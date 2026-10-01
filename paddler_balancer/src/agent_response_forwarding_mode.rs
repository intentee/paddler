use std::time::Duration;

use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::streamable_result::StreamableResult;

use crate::balancer_shutdown_error::balancer_shutdown_error;
use crate::forwarding_event::ForwardingEvent;
use crate::forwarding_plan::ForwardingPlan;

pub enum AgentResponseForwardingMode {
    DrainingUntilAgentConfirms,
    ForwardingToClient,
}

impl AgentResponseForwardingMode {
    #[must_use]
    pub fn plan_for<TResponse: StreamableResult>(
        &self,
        event: ForwardingEvent<TResponse>,
        inference_item_timeout: Duration,
    ) -> ForwardingPlan<TResponse> {
        match self {
            Self::ForwardingToClient => match event {
                ForwardingEvent::AgentConnectionClosed => {
                    ForwardingPlan::ReplyWithErrorThenFinish(JsonRpcError {
                        code: 502,
                        description: "Agent controller connection closed".to_owned(),
                    })
                }
                ForwardingEvent::ClientConnectionClosed => ForwardingPlan::StopAgent,
                ForwardingEvent::ItemTimedOut => {
                    let timeout_ms = inference_item_timeout.as_millis();

                    ForwardingPlan::ReplyWithErrorThenStopAgent(JsonRpcError {
                        code: 504,
                        description: format!(
                            "Inference timed out after {timeout_ms}ms waiting for next token. \
                            Increase --inference-item-timeout if the prompt requires longer processing."
                        ),
                    })
                }
                ForwardingEvent::ResponseReceived(response) => ForwardingPlan::ForwardResponse {
                    is_done: response.is_done(),
                    response,
                },
                ForwardingEvent::ShutdownRequested => {
                    ForwardingPlan::ReplyWithErrorThenStopAgentThenFinish(balancer_shutdown_error())
                }
            },
            Self::DrainingUntilAgentConfirms => match event {
                ForwardingEvent::ResponseReceived(response) if !response.is_done() => {
                    ForwardingPlan::IgnoreDrainedResponse
                }
                ForwardingEvent::AgentConnectionClosed
                | ForwardingEvent::ClientConnectionClosed
                | ForwardingEvent::ItemTimedOut
                | ForwardingEvent::ResponseReceived(_)
                | ForwardingEvent::ShutdownRequested => ForwardingPlan::Finish,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

    use super::AgentResponseForwardingMode;
    use crate::forwarding_event::ForwardingEvent;
    use crate::forwarding_plan::ForwardingPlan;

    const INFERENCE_ITEM_TIMEOUT: Duration = Duration::from_millis(250);

    fn forwarding(
        event: ForwardingEvent<GeneratedTokenResult>,
    ) -> ForwardingPlan<GeneratedTokenResult> {
        AgentResponseForwardingMode::ForwardingToClient.plan_for(event, INFERENCE_ITEM_TIMEOUT)
    }

    fn draining(
        event: ForwardingEvent<GeneratedTokenResult>,
    ) -> ForwardingPlan<GeneratedTokenResult> {
        AgentResponseForwardingMode::DrainingUntilAgentConfirms
            .plan_for(event, INFERENCE_ITEM_TIMEOUT)
    }

    #[test]
    fn tells_the_client_about_a_shutdown_and_stops_the_agent() {
        assert_eq!(
            forwarding(ForwardingEvent::ShutdownRequested),
            ForwardingPlan::ReplyWithErrorThenStopAgentThenFinish(JsonRpcError {
                code: 503,
                description: "balancer is shutting down".to_owned(),
            })
        );
    }

    #[test]
    fn tells_the_client_when_the_agent_connection_closes() {
        assert_eq!(
            forwarding(ForwardingEvent::AgentConnectionClosed),
            ForwardingPlan::ReplyWithErrorThenFinish(JsonRpcError {
                code: 502,
                description: "Agent controller connection closed".to_owned(),
            })
        );
    }

    #[test]
    fn stops_the_agent_when_the_client_goes_away() {
        assert_eq!(
            forwarding(ForwardingEvent::ClientConnectionClosed),
            ForwardingPlan::StopAgent
        );
    }

    #[test]
    fn tells_the_client_about_an_item_timeout_and_stops_the_agent() {
        assert_eq!(
            forwarding(ForwardingEvent::ItemTimedOut),
            ForwardingPlan::ReplyWithErrorThenStopAgent(JsonRpcError {
                code: 504,
                description: "Inference timed out after 250ms waiting for next token. \
                    Increase --inference-item-timeout if the prompt requires longer processing."
                    .to_owned(),
            })
        );
    }

    #[test]
    fn forwards_responses_to_the_client() {
        assert_eq!(
            forwarding(ForwardingEvent::ResponseReceived(
                GeneratedTokenResult::ContentToken("token".to_owned())
            )),
            ForwardingPlan::ForwardResponse {
                is_done: false,
                response: GeneratedTokenResult::ContentToken("token".to_owned()),
            }
        );
    }

    #[test]
    fn ignores_responses_while_the_agent_confirms_the_stop() {
        assert_eq!(
            draining(ForwardingEvent::ResponseReceived(
                GeneratedTokenResult::ContentToken("token".to_owned())
            )),
            ForwardingPlan::IgnoreDrainedResponse
        );
    }

    #[test]
    fn finishes_draining_once_the_agent_confirms_the_stop() {
        assert_eq!(
            draining(ForwardingEvent::ResponseReceived(
                GeneratedTokenResult::ChatTemplateError("template failed".to_owned())
            )),
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
