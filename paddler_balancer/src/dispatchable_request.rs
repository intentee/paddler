use crate::agent_streaming_request::AgentStreamingRequest;
use crate::dispatch_requirement::DispatchRequirement;

pub struct DispatchableRequest<TParams> {
    pub dispatch_requirement: DispatchRequirement,
    pub params: TParams,
}

impl<TParams: AgentStreamingRequest> From<TParams> for DispatchableRequest<TParams> {
    fn from(params: TParams) -> Self {
        Self {
            dispatch_requirement: params.dispatch_requirement(),
            params,
        }
    }
}
