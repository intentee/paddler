use std::future::Future;
use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::rt;
use actix_web::web::Payload;
use actix_ws::AggregatedMessage;
use actix_ws::ProtocolError;
use actix_ws::handle;
use anyhow::Result;
use futures_util::StreamExt as _;
use log::debug;
use log::error;
use serde::de::DeserializeOwned;
use serde_json::Error as SerdeJsonError;
use serde_json::from_str;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;
use tokio::time::MissedTickBehavior;
use tokio::time::interval_at;
use tokio_util::sync::CancellationToken;

use paddler_messaging::ping_interval::PING_INTERVAL;
use paddler_messaging::ping_liveness::PingLiveness;
use paddler_messaging::ping_turn::PingTurn;
use paddler_messaging::rpc_message::RpcMessage;

use crate::continuation_decision::ContinuationDecision;
use crate::enqueue_websocket_frame::enqueue_websocket_frame;
use crate::max_websocket_message_size::MAX_WEBSOCKET_MESSAGE_SIZE;
use crate::websocket_close_cause::WebSocketCloseCause;
use crate::websocket_outbound_frame::WebSocketOutboundFrame;
use crate::websocket_session_controller::WebSocketSessionController;
use crate::write_websocket_session::write_websocket_session;

pub trait ControlsWebSocketEndpoint: Send + Sync + 'static {
    type Context: Send + Sync + 'static;
    type IncomingMessage: DeserializeOwned + RpcMessage + Sync + 'static;
    type OutgoingMessage: RpcMessage + Sync + 'static;

    fn create_context(&self) -> Self::Context;

    fn handle_deserialized_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> impl Future<Output = ContinuationDecision> + Send;

    fn handle_undeserializable_message(
        text: &str,
        deserialization_error: SerdeJsonError,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> impl Future<Output = ContinuationDecision> + Send;

    fn handle_aggregated_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        msg: Option<Result<AggregatedMessage, ProtocolError>>,
        outbound_tx: &UnboundedSender<WebSocketOutboundFrame>,
        ping_liveness: &mut PingLiveness,
        continuation_stop_tx: UnboundedSender<WebSocketCloseCause>,
    ) -> impl Future<Output = ContinuationDecision> + Send {
        async move {
            match msg {
                Some(Ok(AggregatedMessage::Binary(_))) => {
                    debug!("Received binary message, but only text messages are supported");

                    ContinuationDecision::Continue
                }
                Some(Ok(AggregatedMessage::Close(_))) | None => {
                    ContinuationDecision::Stop(WebSocketCloseCause::PeerClosedConnection)
                }
                Some(Ok(AggregatedMessage::Ping(ping_payload))) => {
                    enqueue_websocket_frame(outbound_tx, WebSocketOutboundFrame::Pong(ping_payload))
                }
                Some(Ok(AggregatedMessage::Pong(_))) => {
                    ping_liveness.record_pong();

                    ContinuationDecision::Continue
                }
                Some(Ok(AggregatedMessage::Text(text))) => {
                    Self::handle_text_message(
                        connection_close,
                        context,
                        &text,
                        WebSocketSessionController::<Self::OutgoingMessage>::new(
                            outbound_tx.clone(),
                        ),
                        continuation_stop_tx,
                    )
                    .await
                }
                Some(Err(protocol_error)) => {
                    error!("Error receiving message: {protocol_error:?}");

                    ContinuationDecision::Stop(WebSocketCloseCause::from(&protocol_error))
                }
            }
        }
    }

    fn handle_text_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        text: &str,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
        continuation_stop_tx: UnboundedSender<WebSocketCloseCause>,
    ) -> impl Future<Output = ContinuationDecision> + Send {
        async move {
            match from_str::<Self::IncomingMessage>(text) {
                Ok(deserialized_message) => {
                    rt::spawn(async move {
                        if let ContinuationDecision::Stop(close_cause) =
                            Self::handle_deserialized_message(
                                connection_close,
                                context,
                                deserialized_message,
                                websocket_session_controller,
                            )
                            .await
                            && continuation_stop_tx.send(close_cause).is_err()
                        {
                            debug!("The connection stopped before the handler asked it to stop");
                        }
                    });

                    ContinuationDecision::Continue
                }
                Err(deserialization_error) => {
                    error!(
                        "Paddler-RPC message could not be deserialized: {deserialization_error}"
                    );

                    Self::handle_undeserializable_message(
                        text,
                        deserialization_error,
                        websocket_session_controller,
                    )
                    .await
                }
            }
        }
    }

    fn on_connection_start(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        outbound_tx: &UnboundedSender<WebSocketOutboundFrame>,
    ) -> impl Future<Output = ()> + Send;

    fn respond(
        &self,
        payload: Payload,
        req: HttpRequest,
        shutdown: CancellationToken,
    ) -> Result<HttpResponse, Error> {
        let connection_close = CancellationToken::new();
        let context = Arc::new(self.create_context());
        let (res, session, msg_stream) = handle(&req, payload)?;

        let mut aggregated_msg_stream = msg_stream
            .max_frame_size(MAX_WEBSOCKET_MESSAGE_SIZE)
            .aggregate_continuations()
            .max_continuation_size(MAX_WEBSOCKET_MESSAGE_SIZE);

        rt::spawn(async move {
            let (outbound_tx, outbound_rx) = mpsc::unbounded_channel::<WebSocketOutboundFrame>();

            rt::spawn(write_websocket_session(
                connection_close.clone(),
                outbound_rx,
                session,
            ));

            Self::on_connection_start(connection_close.clone(), context.clone(), &outbound_tx)
                .await;

            let (continuation_stop_tx, mut continuation_stop_rx) =
                mpsc::unbounded_channel::<WebSocketCloseCause>();
            let mut ping_liveness = PingLiveness::default();
            let mut ping_ticker = interval_at(Instant::now() + PING_INTERVAL, PING_INTERVAL);

            ping_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

            let close_cause = loop {
                select! {
                    msg = aggregated_msg_stream.next() => {
                        if let ContinuationDecision::Stop(close_cause) = Self::handle_aggregated_message(
                            connection_close.clone(),
                            context.clone(),
                            msg,
                            &outbound_tx,
                            &mut ping_liveness,
                            continuation_stop_tx.clone(),
                        ).await {
                            break close_cause;
                        }
                    }
                    _ = ping_ticker.tick() => {
                        let ping_decision = match ping_liveness.next_ping() {
                            PingTurn::PeerStoppedAnswering => {
                                ContinuationDecision::Stop(WebSocketCloseCause::PeerStoppedAnsweringPings)
                            }
                            PingTurn::SendPing => {
                                enqueue_websocket_frame(&outbound_tx, WebSocketOutboundFrame::Ping)
                            }
                        };

                        if let ContinuationDecision::Stop(close_cause) = ping_decision {
                            break close_cause;
                        }
                    }
                    Some(close_cause) = continuation_stop_rx.recv() => {
                        break close_cause;
                    }
                    () = connection_close.cancelled() => {
                        break WebSocketCloseCause::ConnectionCloseRequested;
                    }
                    () = shutdown.cancelled() => {
                        break WebSocketCloseCause::ServerShuttingDown;
                    }
                }
            };

            connection_close.cancel();

            let close_frame_delivery =
                outbound_tx.send(WebSocketOutboundFrame::Close(close_cause.close_reason()));

            debug!("Closing the WebSocket session after {close_cause:?}: {close_frame_delivery:?}");
        });

        Ok(res)
    }
}
