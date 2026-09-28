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
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt as _;
use log::debug;
use log::error;
use log::warn;
use paddler_messaging::rpc_message::RpcMessage;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::close_reason_for_protocol_error::close_reason_for_protocol_error;
use crate::continuation_decision::ContinuationDecision;
use crate::continuation_stop_parameters::ContinuationStopParameters;
use crate::max_websocket_frame_size::MAX_WEBSOCKET_FRAME_SIZE;
use crate::websocket_session_controller::WebSocketSessionController;

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
    ) -> ContinuationDecision;

    async fn handle_aggregated_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        msg: Option<Result<AggregatedMessage, ProtocolError>>,
        session: &mut Session,
        continuation_stop_tx: UnboundedSender<ContinuationStopParameters>,
    ) -> ContinuationDecision {
        match msg {
            Some(Ok(AggregatedMessage::Binary(_))) => {
                debug!("Received binary message, but only text messages are supported");

                ContinuationDecision::Continue
            }
            Some(Ok(AggregatedMessage::Close(_))) | None => {
                ContinuationDecision::Stop(ContinuationStopParameters { close_reason: None })
            }
            Some(Ok(AggregatedMessage::Ping(msg))) => {
                if session.pong(&msg).await.is_err() {
                    return ContinuationDecision::Stop(ContinuationStopParameters {
                        close_reason: None,
                    });
                }

                ContinuationDecision::Continue
            }
            Some(Ok(AggregatedMessage::Pong(_))) => ContinuationDecision::Continue,
            Some(Ok(AggregatedMessage::Text(text))) => {
                Self::handle_text_message(
                    connection_close,
                    context,
                    &text,
                    WebSocketSessionController::<Self::OutgoingMessage>::new(session.clone()),
                    continuation_stop_tx,
                );

                ContinuationDecision::Continue
            }
            Some(Err(protocol_error)) => {
                error!("Error receiving message: {protocol_error:?}");

                ContinuationDecision::Stop(ContinuationStopParameters {
                    close_reason: Some(close_reason_for_protocol_error(&protocol_error)),
                })
            }
        }
    }

    fn handle_text_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        text: &str,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
        continuation_stop_tx: UnboundedSender<ContinuationStopParameters>,
    ) {
        match serde_json::from_str::<Self::IncomingMessage>(text) {
            Ok(deserialized_message) => {
                rt::spawn(async move {
                    if let ContinuationDecision::Stop(stop_parameters) =
                        Self::handle_deserialized_message(
                            connection_close,
                            context,
                            deserialized_message,
                            websocket_session_controller,
                        )
                        .await
                        && continuation_stop_tx.send(stop_parameters).is_err()
                    {
                        debug!("The connection stopped before the handler asked it to stop");
                    }
                });
            }
            Err(err) => {
                error!("Paddler-RPC message could not be deserialized: {err}");
            }
        }
    }

    async fn on_connection_start(
        _connection_close: CancellationToken,
        _context: Arc<Self::Context>,
        _session: &mut Session,
    ) {
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
            .max_frame_size(MAX_WEBSOCKET_FRAME_SIZE)
            .aggregate_continuations()
            .max_continuation_size(MAX_CONTINUATION_SIZE);

        rt::spawn(async move {
            let mut close_reason: Option<CloseReason> = None;

            Self::on_connection_start(connection_close.clone(), context.clone(), &mut session)
                .await;

            let (continuation_stop_tx, mut continuation_stop_rx) =
                mpsc::unbounded_channel::<ContinuationStopParameters>();
            let mut ping_ticker = interval(PING_INTERVAL);

            ping_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                tokio::select! {
                    msg = aggregated_msg_stream.next() => {
                        if let ContinuationDecision::Stop(stop_parameters) = Self::handle_aggregated_message(
                            connection_close.clone(),
                            context.clone(),
                            msg,
                            &mut session,
                            continuation_stop_tx.clone(),
                        ).await {
                            close_reason = stop_parameters.close_reason;

                            break;
                        }
                    }
                    Some(stop_parameters) = continuation_stop_rx.recv() => {
                        close_reason = stop_parameters.close_reason;

                        break;
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
