use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::rt;
use actix_web::web::Payload;
use actix_ws::AggregatedMessage;
use actix_ws::Session;
use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt as _;
use log::debug;
use log::error;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::rpc_message::RpcMessage;
use crate::websocket_session_controller::WebSocketSessionController;

const MAX_CONTINUATION_SIZE: usize = 100 * 1024;
const PING_INTERVAL: Duration = Duration::from_secs(3);

pub enum ContinuationDecision {
    Continue,
    Stop,
}

#[async_trait]
pub trait ControlsWebSocketEndpoint: Send + Sync + 'static {
    type Context: Send + Sync + 'static;
    type IncomingMessage: DeserializeOwned + RpcMessage + Sync + 'static;
    type OutgoingMessage: RpcMessage + Sync + 'static;

    fn create_context(&self) -> Self::Context;

    async fn handle_deserialized_message(
        connection_close_tx: broadcast::Sender<()>,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision>;

    async fn handle_aggregated_message(
        connection_close_tx: broadcast::Sender<()>,
        context: Arc<Self::Context>,
        msg: Option<Result<AggregatedMessage, actix_ws::ProtocolError>>,
        session: &mut Session,
    ) -> Result<ContinuationDecision> {
        match msg {
            Some(Ok(AggregatedMessage::Binary(_))) => {
                debug!("Received binary message, but only text messages are supported");

                Ok(ContinuationDecision::Continue)
            }
            Some(Ok(AggregatedMessage::Close(_))) => return Ok(ContinuationDecision::Stop),
            Some(Ok(AggregatedMessage::Ping(msg))) => {
                if session.pong(&msg).await.is_err() {
                    return Ok(ContinuationDecision::Stop);
                }

                Ok(ContinuationDecision::Continue)
            }
            Some(Ok(AggregatedMessage::Pong(_))) => {
                // ignore pong messages
                Ok(ContinuationDecision::Continue)
            }
            Some(Ok(AggregatedMessage::Text(text))) => {
                match Self::handle_text_message(
                    connection_close_tx.clone(),
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
            Some(Err(err)) => {
                error!("Error receiving message: {err:?}");

                return Ok(ContinuationDecision::Stop);
            }
            None => return Ok(ContinuationDecision::Stop),
        }
    }

    async fn handle_serialization_error(
        _connection_close_tx: broadcast::Sender<()>,
        _context: Arc<Self::Context>,
        error: serde_json::Error,
        _websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision> {
        error!("Paddler-RPC serialization error: {error}");

        Ok(ContinuationDecision::Continue)
    }

    async fn handle_text_message(
        connection_close_tx: broadcast::Sender<()>,
        context: Arc<Self::Context>,
        text: &str,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision> {
        match serde_json::from_str::<Self::IncomingMessage>(text) {
            Ok(deserialized_message) => {
                rt::spawn(async move {
                    match Self::handle_deserialized_message(
                        connection_close_tx.clone(),
                        context,
                        deserialized_message,
                        websocket_session_controller,
                    )
                    .await
                    {
                        Ok(ContinuationDecision::Continue) => {
                            // Continue processing messages
                        }
                        Ok(ContinuationDecision::Stop) => {
                            if let Err(close_err) = connection_close_tx.send(()) {
                                error!("Failed to send continuation shutdown signal: {close_err}");
                            }
                        }
                        Err(err) => {
                            error!("Error handling deserialized message: {err:?}");

                            if let Err(close_err) = connection_close_tx.send(()) {
                                error!("Failed to send error shutdown signal: {close_err}");
                            }
                        }
                    }
                });

                Ok(ContinuationDecision::Continue)
            }
            Err(err @ serde_json::Error { .. }) if err.is_data() || err.is_syntax() => {
                error!("JSON-RPC syntax error: {err:?}");

                Self::handle_serialization_error(
                    connection_close_tx,
                    context,
                    err,
                    websocket_session_controller,
                )
                .await
            }
            Err(err) => {
                error!("Error handling JSON-RPC request: {err:?}");

                Self::handle_serialization_error(
                    connection_close_tx,
                    context,
                    err,
                    websocket_session_controller,
                )
                .await
            }
        }
    }

    async fn on_connection_start(
        _context: Arc<Self::Context>,
        _session: &mut Session,
    ) -> Result<ContinuationDecision> {
        Ok(ContinuationDecision::Continue)
    }

    fn respond(&self, payload: Payload, req: HttpRequest) -> Result<HttpResponse, Error> {
        let (connection_close_tx, mut connection_close_rx) = broadcast::channel::<()>(2);
        let context = Arc::new(self.create_context());
        let (res, mut session, msg_stream) = actix_ws::handle(&req, payload)?;

        let mut aggregated_msg_stream = msg_stream
            .aggregate_continuations()
            .max_continuation_size(MAX_CONTINUATION_SIZE);

        rt::spawn(async move {
            match Self::on_connection_start(context.clone(), &mut session).await {
                Ok(ContinuationDecision::Continue) => {}
                Ok(ContinuationDecision::Stop) => return,
                Err(err) => {
                    error!("Error in connection start handler: {err:?}");

                    return;
                }
            }

            let mut ping_ticker = interval(PING_INTERVAL);

            ping_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    msg = aggregated_msg_stream.next() => {
                        match Self::handle_aggregated_message(
                            connection_close_tx.clone(),
                            context.clone(),
                            msg,
                            &mut session,
                        ).await {
                            Ok(ContinuationDecision::Continue) => {
                                // continue processing messages
                            }
                            Ok(ContinuationDecision::Stop) => break,
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
                    _ = connection_close_rx.recv() => {
                        break;
                    }
                }
            }

            if let Err(err) = connection_close_tx.send(()) {
                error!("Failed to send shutdown signal: {err}");
            }

            let _ = session.close(None).await;
        });

        Ok(res)
    }
}
