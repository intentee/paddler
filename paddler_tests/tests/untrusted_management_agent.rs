use std::net::SocketAddr;

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
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::untrusted_agent_socket_client::UntrustedAgentSocketClient;

pub struct UntrustedManagementAgent {
    pub socket_client: UntrustedAgentSocketClient,
}

impl UntrustedManagementAgent {
    pub async fn connect(
        management_addr: SocketAddr,
        agent_id: &str,
    ) -> Result<Self, WebSocketError> {
        UntrustedAgentSocketClient::connect(management_addr, agent_id)
            .await
            .map(|socket_client| Self { socket_client })
    }

    pub async fn connect_registered(
        management_addr: SocketAddr,
        agent_id: &str,
        slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot,
    ) -> Result<Self, ClusterHarnessError> {
        let mut untrusted_management_agent = Self::connect(management_addr, agent_id)
            .await
            .expect("the untrusted agent must reach the agent socket");
        let registration = untrusted_management_agent
            .register(RegisterAgentParams {
                name: Some(agent_id.to_owned()),
                slot_aggregated_status_snapshot,
            })
            .await;

        registration.map(|()| untrusted_management_agent)
    }

    pub async fn next_request(
        &mut self,
    ) -> Result<RequestEnvelope<AgentJsonRpcRequest>, ClusterHarnessError> {
        loop {
            if let AgentJsonRpcMessage::Request(request_envelope) =
                from_str(&self.socket_client.next_text().await?)
                    .expect("the balancer must send a decodable agent message")
            {
                return Ok(request_envelope);
            }
        }
    }

    pub async fn register(
        &mut self,
        register_agent_params: RegisterAgentParams,
    ) -> Result<(), ClusterHarnessError> {
        self.send_management_message(&ManagementJsonRpcMessage::Notification(
            ManagementJsonRpcNotification::RegisterAgent(register_agent_params),
        ))
        .await
    }

    pub async fn report_status(
        &mut self,
        slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot,
    ) -> Result<(), ClusterHarnessError> {
        self.send_management_message(&ManagementJsonRpcMessage::Notification(
            ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                slot_aggregated_status_snapshot,
            }),
        ))
        .await
    }

    pub async fn respond(
        &mut self,
        request_id: String,
        response: AgentJsonRpcResponse,
    ) -> Result<(), ClusterHarnessError> {
        self.send_management_message(&ManagementJsonRpcMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id,
            response,
        }))
        .await
    }

    async fn send_management_message(
        &mut self,
        message: &ManagementJsonRpcMessage,
    ) -> Result<(), ClusterHarnessError> {
        self.socket_client
            .send(Message::text(
                to_string(message).expect("a management message must serialize"),
            ))
            .await
    }
}
