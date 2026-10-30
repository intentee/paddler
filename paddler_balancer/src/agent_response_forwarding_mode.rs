use std::time::Duration;

use paddler_messaging::result_step::ResultStep;

use crate::agent_relay_event::AgentRelayEvent;
use crate::forwarding_event::ForwardingEvent;
use crate::forwarding_plan::ForwardingPlan;
use crate::relay_failure::RelayFailure;

pub enum AgentResponseForwardingMode {
    DrainingUntilAgentConfirms,
    ForwardingToRelay,
}

impl AgentResponseForwardingMode {
    #[must_use]
    pub fn plan_for<TPartial, TFinal>(
        &self,
        event: ForwardingEvent<TPartial, TFinal>,
        inference_item_timeout: Duration,
    ) -> ForwardingPlan<TPartial, TFinal> {
        match self {
            Self::ForwardingToRelay => match event {
                ForwardingEvent::AgentConnectionClosed => ForwardingPlan::NotifyRelayThenFinish(
                    AgentRelayEvent::Failed(RelayFailure::AgentDisconnected),
                ),
                ForwardingEvent::ItemTimedOut => ForwardingPlan::NotifyRelayThenStopAgent(
                    AgentRelayEvent::Failed(RelayFailure::ItemTimedOut {
                        inference_item_timeout,
                    }),
                ),
                ForwardingEvent::RelayAbandoned => ForwardingPlan::StopAgent,
                ForwardingEvent::ResponseReceived(ResultStep::Final(final_result)) => {
                    ForwardingPlan::NotifyRelayThenFinish(AgentRelayEvent::Final(final_result))
                }
                ForwardingEvent::ResponseReceived(ResultStep::Partial(partial_result)) => {
                    ForwardingPlan::NotifyRelay(AgentRelayEvent::Partial(partial_result))
                }
                ForwardingEvent::ShutdownRequested => {
                    ForwardingPlan::NotifyRelayThenStopAgentThenFinish(AgentRelayEvent::Failed(
                        RelayFailure::BalancerShuttingDown,
                    ))
                }
            },
            Self::DrainingUntilAgentConfirms => match event {
                ForwardingEvent::ResponseReceived(ResultStep::Partial(_)) => {
                    ForwardingPlan::IgnoreDrainedResponse
                }
                ForwardingEvent::AgentConnectionClosed
                | ForwardingEvent::ItemTimedOut
                | ForwardingEvent::RelayAbandoned
                | ForwardingEvent::ResponseReceived(ResultStep::Final(_))
                | ForwardingEvent::ShutdownRequested => ForwardingPlan::Finish,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use llama_cpp_bindings_types::TokenUsage;

    use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
    use paddler_messaging::generated_token_partial_result::GeneratedTokenPartialResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::result_step::ResultStep;

    use super::AgentResponseForwardingMode;
    use crate::agent_relay_event::AgentRelayEvent;
    use crate::forwarding_event::ForwardingEvent;
    use crate::forwarding_plan::ForwardingPlan;
    use crate::relay_failure::RelayFailure;

    const INFERENCE_ITEM_TIMEOUT: Duration = Duration::from_millis(250);

    type GeneratedTokenForwardingEvent =
        ForwardingEvent<GeneratedTokenPartialResult, GeneratedTokenFinalResult>;

    type GeneratedTokenForwardingPlan =
        ForwardingPlan<GeneratedTokenPartialResult, GeneratedTokenFinalResult>;

    fn forwarding(event: GeneratedTokenForwardingEvent) -> GeneratedTokenForwardingPlan {
        AgentResponseForwardingMode::ForwardingToRelay.plan_for(event, INFERENCE_ITEM_TIMEOUT)
    }

    fn draining(event: GeneratedTokenForwardingEvent) -> GeneratedTokenForwardingPlan {
        AgentResponseForwardingMode::DrainingUntilAgentConfirms
            .plan_for(event, INFERENCE_ITEM_TIMEOUT)
    }

    fn content_token() -> GeneratedTokenPartialResult {
        GeneratedTokenPartialResult::ContentToken("token".to_owned())
    }

    fn generation_done() -> GeneratedTokenFinalResult {
        GeneratedTokenFinalResult::Done(GenerationSummary {
            finish: GenerationFinish::EndOfGeneration,
            usage: TokenUsage::new(),
        })
    }

    #[test]
    fn tells_the_relay_about_a_shutdown_and_stops_the_agent() {
        assert_eq!(
            forwarding(ForwardingEvent::ShutdownRequested),
            ForwardingPlan::NotifyRelayThenStopAgentThenFinish(AgentRelayEvent::Failed(
                RelayFailure::BalancerShuttingDown
            ))
        );
    }

    #[test]
    fn tells_the_relay_when_the_agent_connection_closes() {
        assert_eq!(
            forwarding(ForwardingEvent::AgentConnectionClosed),
            ForwardingPlan::NotifyRelayThenFinish(AgentRelayEvent::Failed(
                RelayFailure::AgentDisconnected
            ))
        );
    }

    #[test]
    fn stops_the_agent_when_the_relay_is_abandoned() {
        assert_eq!(
            forwarding(ForwardingEvent::RelayAbandoned),
            ForwardingPlan::StopAgent
        );
    }

    #[test]
    fn tells_the_relay_about_an_item_timeout_and_stops_the_agent() {
        assert_eq!(
            forwarding(ForwardingEvent::ItemTimedOut),
            ForwardingPlan::NotifyRelayThenStopAgent(AgentRelayEvent::Failed(
                RelayFailure::ItemTimedOut {
                    inference_item_timeout: INFERENCE_ITEM_TIMEOUT,
                }
            ))
        );
    }

    #[test]
    fn forwards_partial_responses_to_the_relay() {
        assert_eq!(
            forwarding(ForwardingEvent::ResponseReceived(ResultStep::Partial(
                content_token()
            ))),
            ForwardingPlan::NotifyRelay(AgentRelayEvent::Partial(content_token()))
        );
    }

    #[test]
    fn forwards_the_final_response_to_the_relay_and_finishes() {
        assert_eq!(
            forwarding(ForwardingEvent::ResponseReceived(ResultStep::Final(
                generation_done()
            ))),
            ForwardingPlan::NotifyRelayThenFinish(AgentRelayEvent::Final(generation_done()))
        );
    }

    #[test]
    fn ignores_partial_responses_while_the_agent_confirms_the_stop() {
        assert_eq!(
            draining(ForwardingEvent::ResponseReceived(ResultStep::Partial(
                content_token()
            ))),
            ForwardingPlan::IgnoreDrainedResponse
        );
    }

    #[test]
    fn finishes_draining_once_the_agent_confirms_the_stop() {
        assert_eq!(
            draining(ForwardingEvent::ResponseReceived(ResultStep::Final(
                generation_done()
            ))),
            ForwardingPlan::Finish
        );
    }

    #[test]
    fn stops_waiting_for_a_confirmation_that_times_out() {
        assert_eq!(
            draining(ForwardingEvent::ItemTimedOut),
            ForwardingPlan::Finish
        );
    }

    #[test]
    fn stops_draining_when_the_balancer_shuts_down() {
        assert_eq!(
            draining(ForwardingEvent::ShutdownRequested),
            ForwardingPlan::Finish
        );
    }

    #[test]
    fn stops_draining_when_the_agent_connection_closes() {
        assert_eq!(
            draining(ForwardingEvent::AgentConnectionClosed),
            ForwardingPlan::Finish
        );
    }
}
