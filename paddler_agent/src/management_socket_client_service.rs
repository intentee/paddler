use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt as _;
use futures_util::stream::BoxStream;
use futures_util::stream::SelectAll;
use log::error;
use log::info;
use tokio::net::TcpStream;
use tokio::select;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async_with_config;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

use crate::balancer_message_context::BalancerMessageContext;
use crate::last_announced_agent_status::LastAnnouncedAgentStatus;
use crate::management_connection_end::ManagementConnectionEnd;
use crate::management_connection_step::ManagementConnectionStep;
use crate::write_management_message::write_management_message;

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
            .await
            .context("Slot status stopped announcing updates while waiting for idle slots")?;

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
        mut ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        shutdown: CancellationToken,
    ) {
        let slot_aggregated_status = &self.balancer_message_context.slot_aggregated_status;
        let mut update_rx = slot_aggregated_status.subscribe_to_updates();
        let registration_snapshot = slot_aggregated_status.make_snapshot();
        let mut last_announced_agent_status =
            LastAnnouncedAgentStatus::new(registration_snapshot.status.clone());
        let mut pipeline_responses =
            SelectAll::<BoxStream<'static, ManagementJsonRpcMessage>>::new();
        let mut step = ManagementConnectionStep::Write(ManagementJsonRpcMessage::Notification(
            ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                name: self.name.clone(),
                slot_aggregated_status_snapshot: registration_snapshot,
            }),
        ));

        let connection_end = loop {
            step = match step {
                ManagementConnectionStep::Continue => select! {
                    biased;
                    () = shutdown.cancelled() => ManagementConnectionStep::Deregister,
                    incoming_message = ws_stream.next() => match incoming_message {
                        Some(Ok(message)) => self.balancer_message_context.handle_message(message),
                        Some(Err(read_error)) => {
                            error!("Failed to read from the management socket: {read_error}");

                            ManagementConnectionStep::End(ManagementConnectionEnd::IncomingStreamFailed)
                        }
                        None => ManagementConnectionStep::End(ManagementConnectionEnd::BalancerClosedConnection),
                    },
                    Ok(()) = update_rx.changed() => last_announced_agent_status
                        .announce_if_changed(slot_aggregated_status.make_snapshot()),
                    Some(pipeline_response) = pipeline_responses.next() => {
                        ManagementConnectionStep::Write(pipeline_response)
                    }
                },
                ManagementConnectionStep::Deregister => {
                    write_management_message(
                        &mut ws_stream,
                        &ManagementJsonRpcMessage::Notification(
                            ManagementJsonRpcNotification::DeregisterAgent,
                        ),
                        ManagementConnectionStep::End(ManagementConnectionEnd::AgentShuttingDown),
                    )
                    .await
                }
                ManagementConnectionStep::End(connection_end) => break connection_end,
                ManagementConnectionStep::StreamResponses(pipeline_response_stream) => {
                    pipeline_responses.push(pipeline_response_stream);

                    ManagementConnectionStep::Continue
                }
                ManagementConnectionStep::Write(message) => {
                    write_management_message(
                        &mut ws_stream,
                        &message,
                        ManagementConnectionStep::Continue,
                    )
                    .await
                }
            };
        };

        let close_outcome = ws_stream.close(connection_end.close_frame()).await;

        info!("Closed the management connection after {connection_end:?}: {close_outcome:?}");
    }
}

#[async_trait]
impl Service for ManagementSocketClientService {
    fn name(&self) -> &'static str {
        "agent::management_socket_client_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::api_path::ApiPath;

    use super::ManagementSocketClientService;
    use crate::balancer_message_context_fixture::BalancerMessageContextFixture;

    #[tokio::test]
    async fn keep_connection_alive_errors_when_the_balancer_refuses_the_connection() {
        let service = ManagementSocketClientService {
            balancer_message_context: BalancerMessageContextFixture::default().context,
            name: None,
            socket_url: format!("ws://127.0.0.1:1{}", ApiPath::agent_socket("test-agent")),
        };

        assert!(
            service
                .keep_connection_alive(CancellationToken::new())
                .await
                .is_err()
        );
    }
}
