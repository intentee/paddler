use futures_util::SinkExt as _;
use futures_util::StreamExt as _;
use futures_util::future::ready;
use serde_json::from_str;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;

use crate::cluster_harness_error::ClusterHarnessError;

pub struct UntrustedBalancerSocket {
    websocket: WebSocketStream<TcpStream>,
}

impl UntrustedBalancerSocket {
    #[must_use]
    pub const fn new(websocket: WebSocketStream<TcpStream>) -> Self {
        Self { websocket }
    }

    pub async fn next_agent_response(
        &mut self,
    ) -> Result<ResponseEnvelope<AgentJsonRpcResponse>, ClusterHarnessError> {
        self.websocket
            .by_ref()
            .filter_map(|frame| {
                ready(
                    frame
                        .ok()
                        .and_then(|frame| frame.into_text().ok())
                        .and_then(|text| from_str::<ManagementJsonRpcMessage>(&text).ok())
                        .and_then(|agent_message| match agent_message {
                            ManagementJsonRpcMessage::Notification(_) => None,
                            ManagementJsonRpcMessage::Response(response_envelope) => {
                                Some(response_envelope)
                            }
                        }),
                )
            })
            .next()
            .await
            .ok_or(ClusterHarnessError::AgentHungUpBeforeResponding)
    }

    pub async fn send(&mut self, message: Message) -> Result<(), ClusterHarnessError> {
        self.websocket
            .send(message)
            .await
            .map_err(ClusterHarnessError::BalancerSocketSendFailed)
    }

    pub async fn wait_for_the_agent_to_serve(&mut self) -> Result<(), ClusterHarnessError> {
        self.websocket
            .by_ref()
            .filter_map(|frame| {
                ready(
                    frame
                        .ok()
                        .and_then(|frame| frame.into_text().ok())
                        .and_then(|text| from_str::<ManagementJsonRpcMessage>(&text).ok())
                        .filter(|agent_message| {
                            matches!(
                                agent_message,
                                ManagementJsonRpcMessage::Notification(
                                    ManagementJsonRpcNotification::UpdateAgentStatus(
                                        update_agent_status_params
                                    )
                                ) if matches!(
                                    update_agent_status_params
                                        .slot_aggregated_status_snapshot
                                        .status
                                        .runtime,
                                    AgentRuntimeStatus::Serving { .. }
                                )
                            )
                        }),
                )
            })
            .next()
            .await
            .map(|_serving_report| ())
            .ok_or(ClusterHarnessError::AgentHungUpBeforeServing)
    }

    pub async fn wait_for_the_agent_to_hang_up(&mut self) {
        self.websocket.by_ref().for_each(|_frame| ready(())).await;
    }
}
