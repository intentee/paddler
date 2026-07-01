use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use log::error;
use log::info;
use pingora::http::RequestHeader;
use pingora::proxy::ProxyHttp;
use pingora::proxy::Session;
use pingora::upstreams::peer::HttpPeer;
use pingora::Error;
use pingora::ErrorSource;
use pingora::Result;

use crate::balancer::request_context::RequestContext;
use crate::balancer::upstream_peer_pool::UpstreamPeerPool;

static MODEL_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#""model"\s*:\s*"([^"]*)""#).expect("model regex is valid")
});

/// Extract the "model" field from a JSON request body.
///
/// Strategy:
/// 1. If the body contains invalid UTF-8, truncate to the last valid boundary.
/// 2. Try proper JSON parsing first (most reliable, no false positives).
/// 3. If JSON parsing fails, try regex extraction as a fallback.
fn extract_model_from_body(body_bytes: &Bytes) -> Option<String> {
    // Handle invalid UTF-8 by truncating to the last valid boundary
    let effective_bytes = match std::str::from_utf8(body_bytes) {
        Ok(_) => body_bytes.clone(),
        Err(e) => {
            let valid_up_to = e.valid_up_to();
            info!("Invalid UTF-8 in request body. Truncating from {} bytes to {} bytes for model extraction.", body_bytes.len(), valid_up_to);
            body_bytes.slice(0..valid_up_to)
        }
    };

    // Try proper JSON parsing first
    if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&effective_bytes) {
        if let Some(model) = json_value.get("model").and_then(|v| v.as_str()) {
            let model = model.to_string();
            info!("Model in request: {:?}", model);
            return Some(model);
        }
    }

    // Fallback: regex extraction on the raw text
    info!("Failed to parse JSON payload, trying regex extraction");
    let body_str = String::from_utf8_lossy(&effective_bytes);
    if let Some(caps) = MODEL_REGEX.captures(&body_str) {
        if let Some(model_match) = caps.get(1) {
            let model = model_match.as_str().to_string();
            info!("Model via regex: {:?}", model);
            return Some(model);
        }
    }

    info!("Failed to extract model from request body");
    None
}

struct RequestBufferGuard<'a>(&'a AtomicUsize);

impl<'a> RequestBufferGuard<'a> {
    fn increment(length: &'a AtomicUsize, max_buffered_requests: usize) -> Option<Self> {
        if length.load(Ordering::Relaxed) >= max_buffered_requests {
            None
        } else {
            length.fetch_add(1, Ordering::Relaxed);

            Some(Self(length))
        }
    }
}

impl Drop for RequestBufferGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct ProxyService {
    buffered_request_timeout: Duration,
    max_buffered_requests: usize,
    rewrite_host_header: bool,
    check_model: bool,
    slots_endpoint_enable: bool,
    upstream_peer_pool: Arc<UpstreamPeerPool>,
}

impl ProxyService {
    pub fn new(
        rewrite_host_header: bool,
        check_model: bool,
        slots_endpoint_enable: bool,
        upstream_peer_pool: Arc<UpstreamPeerPool>,
        buffered_request_timeout: Duration,
        max_buffered_requests: usize,
    ) -> Self {
        Self {
            rewrite_host_header,
            check_model,
            slots_endpoint_enable,
            upstream_peer_pool,
            buffered_request_timeout,
            max_buffered_requests,
        }
    }
}

#[async_trait]
impl ProxyHttp for ProxyService {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext {
            selected_peer: None,
            slot_taken: false,
            upstream_peer_pool: self.upstream_peer_pool.clone(),
            uses_slots: false,
            requested_model: None,
        }
    }

    fn error_while_proxy(
        &self,
        peer: &HttpPeer,
        session: &mut Session,
        proxy_error: Box<Error>,
        ctx: &mut Self::CTX,
        client_reused: bool,
    ) -> Box<Error> {
        error!("Error while proxying: {proxy_error}");

        if ctx.slot_taken {
            if let Err(err) = ctx.release_slot() {
                error!("Failed to release slot: {err}");

                return Error::new(pingora::InternalError);
            }
        }

        let mut proxy_error_with_context = proxy_error.more_context(format!("Peer: {peer}"));

        // only reused client connections where retry buffer is not truncated
        proxy_error_with_context
            .retry
            .decide_reuse(client_reused && !session.as_ref().retry_buffer_truncated());

        proxy_error_with_context
    }

    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        mut connection_err: Box<Error>,
    ) -> Box<Error> {
        error!("Failed to connect: {connection_err}");

        if let Some(peer) = &ctx.selected_peer {
            match self.upstream_peer_pool.quarantine_peer(&peer.agent_id) {
                Ok(true) => {
                    if let Err(err) = self.upstream_peer_pool.restore_integrity() {
                        error!("Failed to restore integrity: {err}");

                        return Error::new(pingora::InternalError);
                    }

                    // ask server to retry, but try a different best peer
                    ctx.selected_peer = None;
                    connection_err.set_retry(true);
                }
                Ok(false) => {
                    // no need to quarantine for some reason
                }
                Err(err) => {
                    error!("Failed to quarantine peer: {err}");

                    return Error::new(pingora::InternalError);
                }
            }
        }

        connection_err
    }

    fn response_body_filter(
        &self,
        _session: &mut Session,
        _body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        if ctx.slot_taken && end_of_stream {
            if let Err(err) = ctx.release_slot() {
                error!("Failed to release slot: {err}");

                return Err(Error::new(pingora::InternalError));
            }
        }

        Ok(None)
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        ctx.uses_slots = match session.req_header().uri.path() {
            "/slots" => {
                if !self.slots_endpoint_enable {
                    return Err(Error::create(
                        pingora::Custom("Slots endpoint is disabled"),
                        ErrorSource::Downstream,
                        None,
                        None,
                    ));
                }
                false
            }
            "/chat/completions" => true,
            "/completion" => true,
            "/v1/completions" => true,
            "/v1/chat/completions" => true,
            _ => false,
        };

        info!("upstream_peer - {:?} request | rewrite_host_header? {} check_model? {}", session.req_header().method, self.rewrite_host_header, self.check_model);

        // Check if the request method is POST and the content type is JSON
        if self.check_model && ctx.uses_slots {
            info!("Checking model...");
            if session.req_header().method == "POST" {
                // Check if the content type is application/json
                if let Some(content_type) = session.get_header("Content-Type") {
                    if let Ok(content_type_str) = content_type.to_str() {
                        if content_type_str.contains("application/json") {
                            // Enable retry buffering to preserve the request body, reference: https://github.com/cloudflare/pingora/issues/349#issuecomment-2377277028
                            session.enable_retry_buffering();

                            // Read one chunk from the body. The model field is typically
                            // the first field in the JSON, so one read is sufficient.
                            // The retry buffer (64KB) captures what was read.
                            let read_result = session.read_body_or_idle(false).await;
                            match read_result {
                                Ok(Some(_)) => {
                                    // Check if the retry buffer was truncated (body > 64KB).
                                    // If truncated, get_retry_buffer() returns None, so we
                                    // cannot extract the model and must reject the request.
                                    if session.retry_buffer_truncated() {
                                        error!("Request body exceeds 64KB retry buffer limit, cannot determine model");
                                        session
                                            .respond_error(pingora::http::StatusCode::BAD_REQUEST.as_u16())
                                            .await?;
                                        return Err(Error::new_down(pingora::ErrorType::ConnectRefused));
                                    }

                                    if let Some(body_bytes) = session.get_retry_buffer() {
                                        ctx.requested_model = extract_model_from_body(&body_bytes);
                                    } else {
                                        info!("Retry buffer is empty after reading body chunk");
                                    }
                                }
                                Ok(None) => {
                                    info!("Request body is empty");
                                }
                                Err(e) => {
                                    error!("Failed to read request body: {e}");
                                    session
                                        .respond_error(pingora::http::StatusCode::BAD_REQUEST.as_u16())
                                        .await?;
                                    return Err(Error::new_down(pingora::ErrorType::ConnectRefused));
                                }
                            }
                        }
                    }
                }
            }

            // abort if model has not been set
            if ctx.requested_model.is_none() {
                info!("Model missing in request");
                session
                    .respond_error(pingora::http::StatusCode::BAD_REQUEST.as_u16())
                    .await?;

                return Err(Error::new_down(pingora::ErrorType::ConnectRefused));
            } else if !ctx.has_peer_supporting_model() {
                info!("Model {:?} not supported by upstream", ctx.requested_model);
                session
                    .respond_error(pingora::http::StatusCode::NOT_FOUND.as_u16())
                    .await?;

                return Err(Error::new_down(pingora::ErrorType::ConnectRefused));
            } else {
                info!("Model {:?}", ctx.requested_model);
            }
        }

        let peer = tokio::select! {
            result = async {
                loop {
                    ctx.select_upstream_peer()?;

                    if let Some(peer) = ctx.selected_peer.clone() {
                        return Ok::<_, Box<Error>>(peer)
                    }

                    let Some(_req_guard) = RequestBufferGuard::increment(
                        &self.upstream_peer_pool.request_buffer_length,
                        self.max_buffered_requests,
                    ) else {
                        session
                            .respond_error(pingora::http::StatusCode::TOO_MANY_REQUESTS.as_u16())
                            .await?;

                        return Err(Error::create(
                            pingora::ErrorType::ConnectRefused,
                            ErrorSource::Internal,
                            None,
                            None,
                        ));
                    };

                    // To avoid wasting CPU cycles, we don't immediately retry to
                    // `select_upstream_peer` and wait for a notification from code that's
                    // executed when a slot may become available (e.g., the
                    // `/api/v1/agent_status_update/{agent_id}` endpoint).
                    self.upstream_peer_pool.available_slots_notifier.notified().await;
                }
            } => {
                result?
            }
            _ = tokio::time::sleep(self.buffered_request_timeout) => {
                session
                    .respond_error(pingora::http::StatusCode::GATEWAY_TIMEOUT.as_u16())
                    .await?;

                return Err(Error::create(
                    pingora::ErrorType::ConnectTimedout,
                    ErrorSource::Internal,
                    None,
                    None,
                ));
            }
        };

        let mut http_peer = HttpPeer::new(peer.status.external_llamacpp_addr, false, "".into());
        // Expire pooled upstream connections after 30s of inactivity to prevent
        // stale connections from accumulating (idle_timeout was None by default).
        http_peer.options.idle_timeout = Some(Duration::from_secs(30));
        Ok(http_peer.into())
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        if self.rewrite_host_header {
            if let Some(peer) = &ctx.selected_peer {
                upstream_request.insert_header(
                    "Host".to_string(),
                    peer.status.external_llamacpp_addr.to_string(),
                )?;
            }
        }

        Ok(())
    }
}
