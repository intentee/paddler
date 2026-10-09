use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::response::Response as JsonRpcResponse;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;

#[must_use]
pub const fn agent_response_message(
    request_id: String,
    response: JsonRpcResponse,
) -> ManagementJsonRpcMessage {
    ManagementJsonRpcMessage::Response(ResponseEnvelope {
        generated_by: None,
        request_id,
        response,
    })
}
