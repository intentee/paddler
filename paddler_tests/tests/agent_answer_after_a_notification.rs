use serde_json::to_string;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_test_cluster_harness::untrusted_balancer_socket_server::UntrustedBalancerSocketServer;

use crate::agent_runner_connected_to::agent_runner_connected_to;

pub const ANSWERED_REQUEST_ID: &str = "model-metadata";

pub async fn agent_answer_after_a_notification(
    notification: AgentJsonRpcNotification,
) -> ResponseEnvelope<AgentJsonRpcResponse> {
    let untrusted_balancer = UntrustedBalancerSocketServer::bind()
        .await
        .expect("the untrusted balancer must bind");
    let _agent_runner = agent_runner_connected_to(untrusted_balancer.management_addr, 1);
    let mut connection = untrusted_balancer
        .accept_agent()
        .await
        .expect("the agent must connect");

    for agent_message in [
        AgentJsonRpcMessage::Notification(notification),
        AgentJsonRpcMessage::Request(RequestEnvelope {
            id: ANSWERED_REQUEST_ID.to_owned(),
            request: AgentJsonRpcRequest::GetModelMetadata,
        }),
    ] {
        connection
            .send(Message::text(
                to_string(&agent_message).expect("the agent message must serialize"),
            ))
            .await
            .expect("the agent message must be sent");
    }

    connection
        .next_agent_response()
        .await
        .expect("the agent must answer over the same connection")
}
