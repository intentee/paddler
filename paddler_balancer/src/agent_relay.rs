use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::agent_relay_conclusion::AgentRelayConclusion;
use crate::agent_relay_event::AgentRelayEvent;
use crate::agent_relay_phase::AgentRelayPhase;
use crate::agent_relay_step::AgentRelayStep;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::awaiting_agent_relay::AwaitingAgentRelay;
use crate::dispatched_agent::DispatchedAgent;
use crate::dispatched_agent_relay::DispatchedAgentRelay;

pub struct AgentRelay<TParams>
where
    TParams: AgentStreamingRequest,
{
    phase: AgentRelayPhase<TParams>,
    settlement: CancellationToken,
}

impl<TParams> AgentRelay<TParams>
where
    TParams: AgentStreamingRequest,
{
    #[must_use]
    pub fn awaiting(awaiting_agent_relay: AwaitingAgentRelay<TParams>) -> Self {
        let settlement = CancellationToken::new();

        Self {
            phase: AgentRelayPhase::Awaiting {
                awaiting_agent_relay,
                settlement_guard: settlement.clone().drop_guard(),
            },
            settlement,
        }
    }

    #[must_use]
    pub fn dispatched(
        dispatched_agent: DispatchedAgent,
        params: TParams,
        inference_item_timeout: Duration,
        shutdown: CancellationToken,
    ) -> Self {
        let settlement = CancellationToken::new();

        Self {
            phase: AgentRelayPhase::Dispatched(DispatchedAgentRelay::start(
                dispatched_agent,
                params,
                inference_item_timeout,
                settlement.clone(),
                shutdown,
            )),
            settlement,
        }
    }

    #[must_use]
    pub fn settlement(&self) -> CancellationToken {
        self.settlement.clone()
    }

    pub async fn next(self) -> AgentRelayStep<TParams> {
        let Self { phase, settlement } = self;
        let dispatched_agent_relay = match phase {
            AgentRelayPhase::Awaiting {
                awaiting_agent_relay,
                settlement_guard,
            } => match awaiting_agent_relay.dispatch(settlement_guard).await {
                Ok(dispatched_agent_relay) => dispatched_agent_relay,
                Err(relay_failure) => {
                    return AgentRelayStep::Concluded(AgentRelayConclusion::Failed(relay_failure));
                }
            },
            AgentRelayPhase::Dispatched(dispatched_agent_relay) => dispatched_agent_relay,
        };

        match dispatched_agent_relay.agent_events.next().await {
            AgentRelayEvent::Failed(relay_failure) => {
                AgentRelayStep::Concluded(AgentRelayConclusion::Failed(relay_failure))
            }
            AgentRelayEvent::Final(final_result) => {
                AgentRelayStep::Concluded(AgentRelayConclusion::AgentConcluded {
                    final_result,
                    generated_by: dispatched_agent_relay.generated_by,
                })
            }
            AgentRelayEvent::Partial(partial_result) => AgentRelayStep::Partial {
                generated_by: dispatched_agent_relay.generated_by.clone(),
                partial_result,
                relay: Self {
                    phase: AgentRelayPhase::Dispatched(dispatched_agent_relay),
                    settlement,
                },
            },
        }
    }
}
