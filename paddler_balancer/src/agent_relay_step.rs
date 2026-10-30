use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_relay::AgentRelay;
use crate::agent_relay_conclusion::AgentRelayConclusion;
use crate::agent_streaming_request::AgentStreamingRequest;

pub enum AgentRelayStep<TParams>
where
    TParams: AgentStreamingRequest,
{
    Concluded(AgentRelayConclusion<<TParams::Response as StreamableResult>::Final>),
    Partial {
        generated_by: Option<String>,
        partial_result: <TParams::Response as StreamableResult>::Partial,
        relay: AgentRelay<TParams>,
    },
}
