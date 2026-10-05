use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::rt;
use actix_web::web::Payload;
use actix_ws::AggregatedMessage;
use actix_ws::CloseCode;
use actix_ws::CloseReason;
use actix_ws::ProtocolError;
use actix_ws::Session;
use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt as _;
use log::debug;
use log::error;
use log::warn;
use paddler_messaging::rpc_message::RpcMessage;
use serde::de::DeserializeOwned;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::continuation_decision::ContinuationDecision;
use crate::continuation_stop_parameters::ContinuationStopParameters;
use crate::websocket_session_controller::WebSocketSessionController;

const MAX_FRAME_SIZE: usize = 50 * 1024 * 1024;
const MAX_CONTINUATION_SIZE: usize = 50 * 1024 * 1024;
const PING_INTERVAL: Duration = Duration::from_secs(3);

#[async_trait]
pub trait ControlsWebSocketEndpoint: Send + Sync + 'static {
    type Context: Send + Sync + 'static;
    type IncomingMessage: DeserializeOwned + RpcMessage + Sync + 'static;
    type OutgoingMessage: RpcMessage + Sync + 'static;

    fn create_context(&self) -> Self::Context;

    async fn handle_deserialized_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision>;

    async fn handle_aggregated_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        msg: Option<Result<AggregatedMessage, actix_ws::ProtocolError>>,
        session: &mut Session,
    ) -> Result<ContinuationDecision> {
        match msg {
            Some(Ok(AggregatedMessage::Binary(_))) => {
                debug!("Received binary message, but only text messages are supported");

                Ok(ContinuationDecision::Continue)
            }
            Some(Ok(AggregatedMessage::Close(_))) | None => {
                return Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                    close_reason: None,
                }));
            }
            Some(Ok(AggregatedMessage::Ping(msg))) => {
                if session.pong(&msg).await.is_err() {
                    return Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                        close_reason: None,
                    }));
                }

                Ok(ContinuationDecision::Continue)
            }
            Some(Ok(AggregatedMessage::Pong(_))) => {
                // ignore pong messages
                Ok(ContinuationDecision::Continue)
            }
            Some(Ok(AggregatedMessage::Text(text))) => {
                match Self::handle_text_message(
                    connection_close,
                    context.clone(),
                    &text,
                    WebSocketSessionController::<Self::OutgoingMessage>::new(session.clone()),
                )
                .await
                .context(format!("Text message: {text}"))
                {
                    Ok(continuation_decision) => return Ok(continuation_decision),
                    Err(err) => {
                        error!("Error handling text message: {err:?}");

                        Ok(ContinuationDecision::Continue)
                    }
                }
            }
            Some(Err(ProtocolError::Overflow)) => {
                error!("Message exceeded the maximum allowed frame size of {MAX_FRAME_SIZE} bytes");

                return Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                    close_reason: Some(CloseReason {
                        code: CloseCode::Size,
                        description: Some(format!(
                            "Message exceeded the maximum allowed frame size of {MAX_FRAME_SIZE} bytes"
                        )),
                    }),
                }));
            }
            Some(Err(ProtocolError::Io(ref io_err)))
                if io_err
                    .to_string()
                    .contains("Exceeded maximum continuation size") =>
            {
                error!(
                    "Message exceeded the maximum allowed continuation size of {MAX_CONTINUATION_SIZE} bytes"
                );

                return Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                    close_reason: Some(CloseReason {
                        code: CloseCode::Size,
                        description: Some(format!(
                            "Message exceeded the maximum allowed continuation size of {MAX_CONTINUATION_SIZE} bytes"
                        )),
                    }),
                }));
            }
            Some(Err(err)) => {
                error!("Error receiving message: {err:?}");

                return Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                    close_reason: None,
                }));
            }
        }
    }

    async fn handle_serialization_error(
        _connection_close: CancellationToken,
        _context: Arc<Self::Context>,
        error: serde_json::Error,
        _websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision> {
        error!("Paddler-RPC serialization error: {error}");

        Ok(ContinuationDecision::Continue)
    }

    async fn handle_text_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        text: &str,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision> {
        match serde_json::from_str::<Self::IncomingMessage>(text) {
            Ok(deserialized_message) => {
                rt::spawn(async move {
                    match Self::handle_deserialized_message(
                        connection_close.clone(),
                        context,
                        deserialized_message,
                        websocket_session_controller,
                    )
                    .await
                    {
                        Ok(ContinuationDecision::Continue) => {
                            // Continue processing messages
                        }
                        Ok(ContinuationDecision::Stop(_)) => connection_close.cancel(),
                        Err(err) => {
                            error!("Error handling deserialized message: {err:?}");

                            connection_close.cancel();
                        }
                    }
                });

                Ok(ContinuationDecision::Continue)
            }
            Err(err @ serde_json::Error { .. }) if err.is_data() || err.is_syntax() => {
                error!("JSON-RPC syntax error: {err:?}");

                Self::handle_serialization_error(
                    connection_close,
                    context,
                    err,
                    websocket_session_controller,
                )
                .await
            }
            Err(err) => {
                error!("Error handling JSON-RPC request: {err:?}");

                Self::handle_serialization_error(
                    connection_close,
                    context,
                    err,
                    websocket_session_controller,
                )
                .await
            }
        }
    }

    async fn on_connection_start(
        _connection_close: CancellationToken,
        _context: Arc<Self::Context>,
        _session: &mut Session,
    ) -> Result<ContinuationDecision> {
        Ok(ContinuationDecision::Continue)
    }

    fn respond(
        &self,
        payload: Payload,
        req: HttpRequest,
        shutdown: CancellationToken,
    ) -> Result<HttpResponse, Error> {
        let connection_close = CancellationToken::new();
        let context = Arc::new(self.create_context());
        let (res, mut session, msg_stream) = actix_ws::handle(&req, payload)?;

        let mut aggregated_msg_stream = msg_stream
            .max_frame_size(MAX_FRAME_SIZE)
            .aggregate_continuations()
            .max_continuation_size(MAX_CONTINUATION_SIZE);

        rt::spawn(async move {
            let mut close_reason: Option<CloseReason> = None;

            match Self::on_connection_start(connection_close.clone(), context.clone(), &mut session)
                .await
            {
                Ok(ContinuationDecision::Continue) => {}
                Ok(ContinuationDecision::Stop(stop_parameters)) => {
                    close_reason = stop_parameters.close_reason;
                    connection_close.cancel();

                    if let Err(close_err) = session.close(close_reason).await {
                        warn!(
                            "WebSocket session close failed after Stop decision (peer likely already disconnected): {close_err:?}"
                        );
                    }

                    return;
                }
                Err(err) => {
                    error!("Error in connection start handler: {err:?}");
                    connection_close.cancel();

                    if let Err(close_err) = session.close(close_reason).await {
                        warn!(
                            "WebSocket session close failed after start-handler error (peer likely already disconnected): {close_err:?}"
                        );
                    }

                    return;
                }
            }
            let mut ping_ticker = interval(PING_INTERVAL);

            ping_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    msg = aggregated_msg_stream.next() => {
                        match Self::handle_aggregated_message(
                            connection_close.clone(),
                            context.clone(),
                            msg,
                            &mut session,
                        ).await {
                            Ok(ContinuationDecision::Continue) => {
                                // continue processing messages
                            }
                            Ok(ContinuationDecision::Stop(stop_parameters)) => {
                                close_reason = stop_parameters.close_reason;

                                break;
                            }
                            Err(err) => {
                                error!("Error handling aggregated message: {err:?}");

                                break;
                            },
                        }
                    }
                    _ = ping_ticker.tick() => {
                        if session.ping(b"").await.is_err() {
                            break;
                        }
                    }
                    () = connection_close.cancelled() => {
                        break;
                    }
                    () = shutdown.cancelled() => {
                        close_reason = Some(CloseReason {
                            code: CloseCode::Away,
                            description: Some("Server shutting down".to_owned()),
                        });
                        break;
                    }
                }
            }

            connection_close.cancel();

            if let Err(close_err) = session.close(close_reason).await {
                warn!(
                    "WebSocket session close failed at end of message loop (peer likely already disconnected): {close_err:?}"
                );
            }
        });

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use actix_web::FromRequest as _;
    use actix_web::body::to_bytes;
    use actix_web::http::header;
    use actix_web::test::TestRequest;
    use actix_web::web::Bytes;
    use actix_web::web::Payload;
    use serde::Deserialize;
    use serde::Serialize;
    use std::mem::discriminant;

    use super::ContinuationDecision;
    use super::ContinuationStopParameters;
    use super::ControlsWebSocketEndpoint;
    use super::WebSocketSessionController;
    use actix_ws::AggregatedMessage;
    use actix_ws::CloseCode;
    use actix_ws::CloseReason;
    use actix_ws::Session;
    use anyhow::Result;
    use anyhow::anyhow;
    use async_trait::async_trait;
    use paddler_messaging::rpc_message::RpcMessage;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

    #[derive(Deserialize, Serialize)]
    struct ProbeIncomingMessage {}

    impl RpcMessage for ProbeIncomingMessage {}

    #[derive(Serialize)]
    struct ProbeOutgoingMessage;

    impl RpcMessage for ProbeOutgoingMessage {}

    #[derive(Clone, Copy)]
    enum DeserializedMessageOutcome {
        Continue,
        Stop,
        Err,
    }

    struct ProbeEndpoint {
        deserialized_message_outcome: DeserializedMessageOutcome,
    }

    #[async_trait]
    impl ControlsWebSocketEndpoint for ProbeEndpoint {
        type Context = DeserializedMessageOutcome;
        type IncomingMessage = ProbeIncomingMessage;
        type OutgoingMessage = ProbeOutgoingMessage;

        fn create_context(&self) -> Self::Context {
            self.deserialized_message_outcome
        }

        async fn handle_deserialized_message(
            _connection_close: CancellationToken,
            context: Arc<Self::Context>,
            _deserialized_message: Self::IncomingMessage,
            _websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
        ) -> Result<ContinuationDecision> {
            match *context {
                DeserializedMessageOutcome::Continue => Ok(ContinuationDecision::Continue),
                DeserializedMessageOutcome::Stop => {
                    Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                        close_reason: None,
                    }))
                }
                DeserializedMessageOutcome::Err => {
                    Err(anyhow!("deserialized message handler failed"))
                }
            }
        }
    }

    #[derive(Clone, Copy)]
    enum ConnectionStartOutcome {
        Stop,
        Err,
    }

    struct StartOverridingEndpoint {
        connection_start_outcome: ConnectionStartOutcome,
    }

    #[async_trait]
    impl ControlsWebSocketEndpoint for StartOverridingEndpoint {
        type Context = ConnectionStartOutcome;
        type IncomingMessage = ProbeIncomingMessage;
        type OutgoingMessage = ProbeOutgoingMessage;

        fn create_context(&self) -> Self::Context {
            self.connection_start_outcome
        }

        async fn handle_deserialized_message(
            _connection_close: CancellationToken,
            _context: Arc<Self::Context>,
            _deserialized_message: Self::IncomingMessage,
            _websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
        ) -> Result<ContinuationDecision> {
            Ok(ContinuationDecision::Continue)
        }

        async fn on_connection_start(
            _connection_close: CancellationToken,
            context: Arc<Self::Context>,
            _session: &mut Session,
        ) -> Result<ContinuationDecision> {
            match *context {
                ConnectionStartOutcome::Stop => {
                    Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                        close_reason: Some(CloseReason {
                            code: CloseCode::Normal,
                            description: Some("stop on start".to_owned()),
                        }),
                    }))
                }
                ConnectionStartOutcome::Err => Err(anyhow!("connection start handler failed")),
            }
        }
    }

    struct AggregatedErroringEndpoint;

    #[async_trait]
    impl ControlsWebSocketEndpoint for AggregatedErroringEndpoint {
        type Context = ();
        type IncomingMessage = ProbeIncomingMessage;
        type OutgoingMessage = ProbeOutgoingMessage;

        fn create_context(&self) -> Self::Context {}

        async fn handle_deserialized_message(
            _connection_close: CancellationToken,
            _context: Arc<Self::Context>,
            _deserialized_message: Self::IncomingMessage,
            _websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
        ) -> Result<ContinuationDecision> {
            Ok(ContinuationDecision::Continue)
        }

        async fn handle_aggregated_message(
            _connection_close: CancellationToken,
            _context: Arc<Self::Context>,
            _msg: Option<Result<AggregatedMessage, actix_ws::ProtocolError>>,
            _session: &mut Session,
        ) -> Result<ContinuationDecision> {
            Err(anyhow!("aggregated message handler failed"))
        }
    }

    fn handshake_request() -> TestRequest {
        TestRequest::get()
            .insert_header((header::CONNECTION, "upgrade"))
            .insert_header((header::UPGRADE, "websocket"))
            .insert_header((header::SEC_WEBSOCKET_VERSION, "13"))
            .insert_header((header::SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ=="))
    }

    async fn open_session() -> Session {
        let (request, mut raw_payload) = handshake_request().to_http_parts();
        let payload = Payload::from_request(&request, &mut raw_payload)
            .await
            .unwrap();
        let (_response, session, _msg_stream) = actix_ws::handle(&request, payload).unwrap();

        session
    }

    #[actix_web::test]
    async fn handle_serialization_error_continues() {
        let session = open_session().await;
        let serialization_error = serde_json::from_str::<u8>("not-a-number").err().unwrap();
        let continuation_decision = ProbeEndpoint::handle_serialization_error(
            CancellationToken::new(),
            Arc::new(DeserializedMessageOutcome::Continue),
            serialization_error,
            WebSocketSessionController::new(session),
        )
        .await
        .unwrap();

        assert!(matches!(
            continuation_decision,
            ContinuationDecision::Continue
        ));
    }

    #[actix_web::test]
    async fn default_on_connection_start_continues() {
        let mut session = open_session().await;
        let continuation_decision = ProbeEndpoint::on_connection_start(
            CancellationToken::new(),
            Arc::new(DeserializedMessageOutcome::Continue),
            &mut session,
        )
        .await
        .unwrap();

        assert!(matches!(
            continuation_decision,
            ContinuationDecision::Continue
        ));
    }

    #[actix_web::test]
    async fn deserialized_message_stop_cancels_connection() {
        let session = open_session().await;
        let connection_close = CancellationToken::new();
        let continuation_decision = ProbeEndpoint::handle_text_message(
            connection_close.clone(),
            Arc::new(DeserializedMessageOutcome::Stop),
            "{}",
            WebSocketSessionController::new(session),
        )
        .await
        .unwrap();

        assert!(matches!(
            continuation_decision,
            ContinuationDecision::Continue
        ));

        connection_close.cancelled().await;

        assert!(connection_close.is_cancelled());
    }

    #[actix_web::test]
    async fn deserialized_message_error_cancels_connection() {
        let session = open_session().await;
        let connection_close = CancellationToken::new();
        let continuation_decision = ProbeEndpoint::handle_text_message(
            connection_close.clone(),
            Arc::new(DeserializedMessageOutcome::Err),
            "{}",
            WebSocketSessionController::new(session),
        )
        .await
        .unwrap();

        assert!(matches!(
            continuation_decision,
            ContinuationDecision::Continue
        ));

        connection_close.cancelled().await;

        assert!(connection_close.is_cancelled());
    }

    async fn drain_close_frame(endpoint: &impl ControlsWebSocketEndpoint) -> Bytes {
        let (request, mut raw_payload) = handshake_request().to_http_parts();
        let payload = Payload::from_request(&request, &mut raw_payload)
            .await
            .unwrap();
        let response = endpoint
            .respond(payload, request, CancellationToken::new())
            .unwrap();

        assert_eq!(response.status().as_u16(), 101);

        to_bytes(response.into_body()).await.unwrap()
    }

    #[actix_web::test]
    async fn respond_runs_message_loop_until_stream_closes() {
        let close_frame = drain_close_frame(&ProbeEndpoint {
            deserialized_message_outcome: DeserializedMessageOutcome::Continue,
        })
        .await;

        assert!(!close_frame.is_empty());
    }

    #[actix_web::test]
    async fn respond_closes_when_start_handler_stops() {
        let close_frame = drain_close_frame(&StartOverridingEndpoint {
            connection_start_outcome: ConnectionStartOutcome::Stop,
        })
        .await;

        assert!(!close_frame.is_empty());
    }

    #[actix_web::test]
    async fn respond_closes_when_start_handler_errors() {
        let close_frame = drain_close_frame(&StartOverridingEndpoint {
            connection_start_outcome: ConnectionStartOutcome::Err,
        })
        .await;

        assert!(!close_frame.is_empty());
    }

    #[actix_web::test]
    async fn respond_breaks_when_aggregated_handler_errors() {
        let close_frame = drain_close_frame(&AggregatedErroringEndpoint).await;

        assert!(!close_frame.is_empty());
    }

    #[actix_web::test]
    async fn start_overriding_endpoint_deserialized_message_continues() {
        let session = open_session().await;
        let continuation_decision = StartOverridingEndpoint::handle_deserialized_message(
            CancellationToken::new(),
            Arc::new(ConnectionStartOutcome::Stop),
            ProbeIncomingMessage {},
            WebSocketSessionController::new(session),
        )
        .await
        .unwrap();

        assert_eq!(
            discriminant(&continuation_decision),
            discriminant(&ContinuationDecision::Continue)
        );
    }

    #[actix_web::test]
    async fn aggregated_erroring_endpoint_deserialized_message_continues() {
        let session = open_session().await;
        let continuation_decision = AggregatedErroringEndpoint::handle_deserialized_message(
            CancellationToken::new(),
            Arc::new(()),
            ProbeIncomingMessage {},
            WebSocketSessionController::new(session),
        )
        .await
        .unwrap();

        assert_eq!(
            discriminant(&continuation_decision),
            discriminant(&ContinuationDecision::Continue)
        );
    }

    #[actix_web::test]
    async fn respond_propagates_handshake_error_on_non_websocket_request() {
        let (request, mut raw_payload) = TestRequest::get().to_http_parts();
        let payload = Payload::from_request(&request, &mut raw_payload)
            .await
            .unwrap();
        let respond_result =
            AggregatedErroringEndpoint.respond(payload, request, CancellationToken::new());
        let handshake_error = respond_result.err().unwrap();

        assert_eq!(handshake_error.error_response().status().as_u16(), 400);
    }
}
