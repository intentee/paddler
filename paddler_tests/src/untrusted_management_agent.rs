use std::net::SocketAddr;

use anyhow::Context as _;
use anyhow::Error;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use serde_json::from_str;
use serde_json::to_string;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_test_cluster_harness::untrusted_agent_socket_client::UntrustedAgentSocketClient;

pub struct UntrustedManagementAgent {
    pub socket_client: UntrustedAgentSocketClient,
}

impl UntrustedManagementAgent {
    /// # Errors
    ///
    /// Returns the [`WebSocketError`] of a connection to the agent socket that cannot be established.
    pub async fn connect(
        management_addr: SocketAddr,
        agent_id: &str,
    ) -> Result<Self, WebSocketError> {
        UntrustedAgentSocketClient::connect(management_addr, agent_id)
            .await
            .map(|socket_client| Self { socket_client })
    }

    /// # Errors
    ///
    /// Returns an error when the untrusted agent cannot reach the agent socket, and when its registration cannot be serialized or sent.
    pub async fn connect_registered(
        management_addr: SocketAddr,
        agent_id: &str,
        slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot,
    ) -> Result<Self> {
        Self::connect(management_addr, agent_id)
            .map(|connected| connected.context("the untrusted agent must reach the agent socket"))
            .and_then(|mut untrusted_management_agent| async move {
                untrusted_management_agent
                    .register(RegisterAgentParams {
                        name: Some(agent_id.to_owned()),
                        slot_aggregated_status_snapshot,
                    })
                    .await
                    .map(|()| untrusted_management_agent)
            })
            .await
    }

    /// # Errors
    ///
    /// Returns the [`ClusterHarnessError`](paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError) of the agent socket when the balancer sends no text, and an error when the balancer sends an agent message that cannot be decoded.
    pub async fn next_request(&mut self) -> Result<RequestEnvelope<AgentJsonRpcRequest>> {
        loop {
            if let AgentJsonRpcMessage::Request(request_envelope) =
                self.next_agent_message().await?
            {
                return Ok(request_envelope);
            }
        }
    }

    /// # Errors
    ///
    /// Returns an error when the registration cannot be serialized or sent.
    pub async fn register(&mut self, register_agent_params: RegisterAgentParams) -> Result<()> {
        self.send_management_message(&ManagementJsonRpcMessage::Notification(
            ManagementJsonRpcNotification::RegisterAgent(register_agent_params),
        ))
        .await
    }

    /// # Errors
    ///
    /// Returns an error when the status update cannot be serialized or sent.
    pub async fn report_status(
        &mut self,
        slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot,
    ) -> Result<()> {
        self.send_management_message(&ManagementJsonRpcMessage::Notification(
            ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                slot_aggregated_status_snapshot,
            }),
        ))
        .await
    }

    /// # Errors
    ///
    /// Returns an error when the response cannot be serialized or sent.
    pub async fn respond(
        &mut self,
        request_id: String,
        response: AgentJsonRpcResponse,
    ) -> Result<()> {
        self.send_management_message(&ManagementJsonRpcMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id,
            response,
        }))
        .await
    }

    async fn next_agent_message(&mut self) -> Result<AgentJsonRpcMessage> {
        self.socket_client
            .next_text()
            .await
            .map_err(Error::from)
            .and_then(|text| {
                from_str(&text).context("the balancer must send a decodable agent message")
            })
    }

    async fn send_management_message(&mut self, message: &ManagementJsonRpcMessage) -> Result<()> {
        ready(to_string(message).context("a management message must serialize"))
            .and_then(|serialized_message| {
                self.socket_client
                    .send(Message::text(serialized_message))
                    .err_into()
            })
            .await
    }
}
