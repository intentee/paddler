use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;

use crate::agent_relay::AgentRelay;
use crate::agent_streaming_request::AgentStreamingRequest;

pub enum GenerationProgressPhase<TParams>
where
    TParams: AgentStreamingRequest<Response = GeneratedTokenResult>,
{
    Relaying {
        agent_relay: AgentRelay<TParams>,
        settlement: CancellationToken,
    },
    ReleasingRelay {
        relay: AgentRelay<TParams>,
        settlement: CancellationToken,
    },
    Settling {
        settlement: CancellationToken,
    },
}
