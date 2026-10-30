use actix_web::HttpResponse;

use paddler_messaging::decision_result::DecisionResult;
use paddler_typesafe_translation::system_one_response_layout::SystemOneResponseLayout;

use crate::agent_relay::AgentRelay;
use crate::agent_relay_conclusion::AgentRelayConclusion;
use crate::agent_relay_step::AgentRelayStep;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::compatibility::typesafe_service::decision_outcome::DecisionOutcome;
use crate::compatibility::typesafe_service::typesafe_decision_failure::TypeSafeDecisionFailure;

pub async fn system_one_http_response<TParams>(
    response_layout: &SystemOneResponseLayout,
    mut agent_relay: AgentRelay<TParams>,
) -> HttpResponse
where
    TParams: AgentStreamingRequest<Response = DecisionResult>,
{
    let mut answers = Vec::new();

    loop {
        match agent_relay.next().await {
            AgentRelayStep::Concluded(AgentRelayConclusion::AgentConcluded {
                final_result,
                ..
            }) => {
                return match DecisionOutcome::from(final_result) {
                    DecisionOutcome::Failed(typesafe_decision_failure) => {
                        typesafe_decision_failure.into_http_response()
                    }
                    DecisionOutcome::Finished(summary) => {
                        match response_layout.respond(&answers, &summary) {
                            Ok(system_one_response) => HttpResponse::Ok().json(system_one_response),
                            Err(translation_error) => {
                                TypeSafeDecisionFailure::from(translation_error)
                                    .into_http_response()
                            }
                        }
                    }
                };
            }
            AgentRelayStep::Concluded(AgentRelayConclusion::Failed(relay_failure)) => {
                return TypeSafeDecisionFailure::from(relay_failure).into_http_response();
            }
            AgentRelayStep::Partial {
                partial_result: answer,
                relay,
                ..
            } => {
                answers.push(answer);
                agent_relay = relay;
            }
        }
    }
}
