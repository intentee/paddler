use futures::stream::Stream;
use futures::stream::unfold;
use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_openai_translation::generation_progress::GenerationProgress;

use crate::agent_relay::AgentRelay;
use crate::agent_relay_conclusion::AgentRelayConclusion;
use crate::agent_relay_event::AgentRelayEvent;
use crate::agent_relay_step::AgentRelayStep;
use crate::agent_streaming_request::AgentStreamingRequest;

enum GenerationProgressPhase<TParams>
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

impl<TParams> GenerationProgressPhase<TParams>
where
    TParams: AgentStreamingRequest<Response = GeneratedTokenResult>,
{
    async fn advance(
        self,
    ) -> Option<(
        AgentRelayEvent<GenerationProgress, GeneratedTokenFinalResult>,
        Self,
    )> {
        match self {
            Self::Relaying {
                agent_relay,
                settlement,
            } => match agent_relay.next().await {
                AgentRelayStep::Concluded(AgentRelayConclusion::AgentConcluded {
                    final_result,
                    ..
                }) => Some((
                    AgentRelayEvent::Final(final_result),
                    Self::Settling { settlement },
                )),
                AgentRelayStep::Concluded(AgentRelayConclusion::Failed(relay_failure)) => Some((
                    AgentRelayEvent::Failed(relay_failure),
                    Self::Settling { settlement },
                )),
                AgentRelayStep::Partial {
                    partial_result,
                    relay,
                    ..
                } => match GenerationProgress::from(partial_result) {
                    failed_generation @ GenerationProgress::Failed(_) => Some((
                        AgentRelayEvent::Partial(failed_generation),
                        Self::ReleasingRelay { relay, settlement },
                    )),
                    generation_progress => Some((
                        AgentRelayEvent::Partial(generation_progress),
                        Self::Relaying {
                            agent_relay: relay,
                            settlement,
                        },
                    )),
                },
            },
            Self::ReleasingRelay { relay, settlement } => {
                drop(relay);
                settlement.cancelled().await;

                None
            }
            Self::Settling { settlement } => {
                settlement.cancelled().await;

                None
            }
        }
    }
}

pub fn generation_progress_events<TParams>(
    agent_relay: AgentRelay<TParams>,
) -> impl Stream<Item = AgentRelayEvent<GenerationProgress, GeneratedTokenFinalResult>>
where
    TParams: AgentStreamingRequest<Response = GeneratedTokenResult>,
{
    let settlement = agent_relay.settlement();

    unfold(
        GenerationProgressPhase::Relaying {
            agent_relay,
            settlement,
        },
        GenerationProgressPhase::advance,
    )
}
