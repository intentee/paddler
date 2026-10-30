use tokio_util::sync::DropGuard;

use crate::agent_streaming_request::AgentStreamingRequest;
use crate::awaiting_agent_relay::AwaitingAgentRelay;
use crate::dispatched_agent_relay::DispatchedAgentRelay;

pub enum AgentRelayPhase<TParams>
where
    TParams: AgentStreamingRequest,
{
    Awaiting {
        awaiting_agent_relay: AwaitingAgentRelay<TParams>,
        settlement_guard: DropGuard,
    },
    Dispatched(DispatchedAgentRelay<TParams::Response>),
}
