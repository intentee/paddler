use anyhow::Result;
use futures_util::TryFutureExt as _;

use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;

use crate::agent_connected_to_an_untrusted_balancer::AgentConnectedToAnUntrustedBalancer;
use crate::constants::ANSWERED_REQUEST_ID;

/// # Errors
///
/// Returns an error when the agent cannot connect to the untrusted balancer, when the notification or the request cannot be sent, and when the agent does not answer over the same connection.
pub async fn agent_answer_after_a_notification(
    notification: AgentJsonRpcNotification,
) -> Result<ResponseEnvelope<AgentJsonRpcResponse>> {
    AgentConnectedToAnUntrustedBalancer::connect()
        .and_then(|connected_agent| {
            connected_agent.send_agent_message(AgentJsonRpcMessage::Notification(notification))
        })
        .and_then(|connected_agent| {
            connected_agent.send_agent_message(AgentJsonRpcMessage::Request(RequestEnvelope {
                id: ANSWERED_REQUEST_ID.to_owned(),
                request: AgentJsonRpcRequest::GetModelMetadata,
            }))
        })
        .and_then(AgentConnectedToAnUntrustedBalancer::next_agent_response)
        .await
}
