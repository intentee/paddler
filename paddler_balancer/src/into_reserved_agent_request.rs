use std::num::NonZeroU16;

use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::reserved_request::ReservedRequest;

pub trait IntoReservedAgentRequest: Sized {
    fn into_reserved_agent_request(self, reserved_slots: NonZeroU16) -> AgentJsonRpcRequest;
}

impl<TParams> IntoReservedAgentRequest for TParams
where
    ReservedRequest<TParams>: Into<AgentJsonRpcRequest>,
{
    fn into_reserved_agent_request(self, reserved_slots: NonZeroU16) -> AgentJsonRpcRequest {
        ReservedRequest {
            params: self,
            reserved_slots,
        }
        .into()
    }
}
