use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_relay_conclusion::AgentRelayConclusion;

pub enum AgentRelayStep<TResponse, TRelay>
where
    TResponse: StreamableResult,
{
    Concluded(AgentRelayConclusion<TResponse::Final>),
    Partial {
        generated_by: Option<String>,
        partial_result: TResponse::Partial,
        relay: TRelay,
    },
}
