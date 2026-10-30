use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt as _;
use futures_util::stream::BoxStream;
use futures_util::stream::SelectAll;
use futures_util::stream::SplitStream;
use log::error;
use log::info;
use tokio::join;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::Instant;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio::time::interval_at;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_frame_writer::frame_writing_end::FrameWritingEnd;
use paddler_frame_writer::write_frames_until_connection_closes::write_frames_until_connection_closes;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::ping_interval::PING_INTERVAL;
use paddler_messaging::ping_liveness::PingLiveness;
use paddler_messaging::ping_turn::PingTurn;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

use crate::balancer_message_context::BalancerMessageContext;
use crate::enqueue_management_message::enqueue_management_message;
use crate::enqueue_outbound_frame::enqueue_outbound_frame;
use crate::last_announced_agent_status::LastAnnouncedAgentStatus;
use crate::management_connection_end::ManagementConnectionEnd;
use crate::management_connection_step::ManagementConnectionStep;

const BALANCER_RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

pub struct ManagementSocketClientService {
    pub balancer_message_context: BalancerMessageContext,
    pub name: Option<String>,
    pub socket_url: String,
}

impl ManagementSocketClientService {
    async fn connect_once_slots_are_idle(
        &self,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        self.balancer_message_context
            .slot_aggregated_status
            .wait_until_no_slots_are_processing()
            .await;

        info!("Connecting to management server at {}", self.socket_url);

        let (ws_stream, _response) =
            connect_async_with_config(self.socket_url.clone(), None, true).await?;

        info!("Connected to management server");

        Ok(ws_stream)
    }

    async fn keep_connection_alive(&self, shutdown: CancellationToken) -> Result<()> {
        if let Some(connect_outcome) = shutdown
            .run_until_cancelled(self.connect_once_slots_are_idle())
            .await
        {
            self.serve_connection(connect_outcome?, shutdown).await;
        }

        Ok(())
    }

    async fn serve_connection(
        &self,
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        shutdown: CancellationToken,
    ) {
        let (ws_sink, ws_source) = ws_stream.split();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let connection_close = CancellationToken::new();

        let ((), frame_writing_end) = join!(
            self.read_until_the_connection_ends(
                connection_close.clone(),
                ws_source,
                outbound_tx,
                shutdown
            ),
            write_frames_until_connection_closes(connection_close, outbound_rx, ws_sink),
        );

        match frame_writing_end {
            FrameWritingEnd::ConnectionClosed => {
                info!("Abandoned the management socket frames the balancer did not read");
            }
            FrameWritingEnd::EveryFrameSent => {}
            FrameWritingEnd::SendFailed(send_error) => {
                error!("Failed to send a management socket frame: {send_error}");
            }
        }
    }

    async fn read_until_the_connection_ends(
        &self,
        connection_close: CancellationToken,
        mut ws_source: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        outbound_tx: mpsc::UnboundedSender<Message>,
        shutdown: CancellationToken,
    ) {
        let slot_aggregated_status = &self.balancer_message_context.slot_aggregated_status;
        let mut update_rx = slot_aggregated_status.subscribe_to_updates();
        let registration_snapshot = slot_aggregated_status.make_snapshot();
        let mut last_announced_agent_status =
            LastAnnouncedAgentStatus::new(registration_snapshot.status.clone());
        let mut pipeline_responses =
            SelectAll::<BoxStream<'static, ManagementJsonRpcMessage>>::new();
        let mut ping_ticker = interval_at(Instant::now() + PING_INTERVAL, PING_INTERVAL);
        let mut ping_liveness = PingLiveness::default();
        let mut step = ManagementConnectionStep::Write(ManagementJsonRpcMessage::Notification(
            ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                name: self.name.clone(),
                slot_aggregated_status_snapshot: registration_snapshot,
            }),
        ));

        ping_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let connection_end = loop {
            step = match step {
                ManagementConnectionStep::Continue => select! {
                    biased;
                    () = shutdown.cancelled() => ManagementConnectionStep::Deregister,
                    incoming_message = ws_source.next() => match incoming_message {
                        Some(Ok(Message::Pong(_))) => {
                            ping_liveness.record_pong();

                            ManagementConnectionStep::Continue
                        }
                        Some(Ok(message)) => self.balancer_message_context.handle_message(message),
                        Some(Err(read_error)) => {
                            error!("Failed to read from the management socket: {read_error}");

                            ManagementConnectionStep::End(ManagementConnectionEnd::IncomingStreamFailed)
                        }
                        None => ManagementConnectionStep::End(ManagementConnectionEnd::BalancerClosedConnection),
                    },
                    _ = ping_ticker.tick() => match ping_liveness.next_ping() {
                        PingTurn::PeerStoppedAnswering => {
                            ManagementConnectionStep::End(ManagementConnectionEnd::BalancerStoppedAnsweringPings)
                        }
                        PingTurn::SendPing => enqueue_outbound_frame(
                            &outbound_tx,
                            Message::Ping(Bytes::new()),
                            ManagementConnectionStep::Continue,
                        ),
                    },
                    Ok(()) = update_rx.changed() => last_announced_agent_status
                        .announce_if_changed(slot_aggregated_status.make_snapshot()),
                    Some(pipeline_response) = pipeline_responses.next() => {
                        ManagementConnectionStep::Write(pipeline_response)
                    }
                },
                ManagementConnectionStep::Deregister => enqueue_management_message(
                    &outbound_tx,
                    &ManagementJsonRpcMessage::Notification(
                        ManagementJsonRpcNotification::DeregisterAgent,
                    ),
                    ManagementConnectionStep::End(ManagementConnectionEnd::AgentShuttingDown),
                ),
                ManagementConnectionStep::End(connection_end) => break connection_end,
                ManagementConnectionStep::StreamResponses(pipeline_response_stream) => {
                    pipeline_responses.push(pipeline_response_stream);

                    ManagementConnectionStep::Continue
                }
                ManagementConnectionStep::Write(message) => enqueue_management_message(
                    &outbound_tx,
                    &message,
                    ManagementConnectionStep::Continue,
                ),
            };
        };

        let close_frame_delivery = outbound_tx.send(Message::Close(connection_end.close_frame()));

        connection_close.cancel();

        info!(
            "Closing the management connection after {connection_end:?}: {close_frame_delivery:?}"
        );
    }
}

impl ManagementSocketClientService {
    async fn reconnect_to_the_balancer_until_shutdown(
        self: Box<Self>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let mut ticker = interval(BALANCER_RECONNECT_INTERVAL);

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            select! {
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    match self.keep_connection_alive(shutdown.clone()).await {
                        Err(err) => {
                            error!("Failed to keep the connection alive: {err:?}");
                        }
                        Ok(()) => {
                            info!("Management server connection closed");
                        }
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Service for ManagementSocketClientService {
    fn name(&self) -> &'static str {
        "agent::management_socket_client_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        self.reconnect_to_the_balancer_until_shutdown(shutdown)
            .await
    }
}
