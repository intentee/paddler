use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::post;

use paddler_balancer::agent_relay::AgentRelay;
use paddler_balancer::agent_relay_conclusion::AgentRelayConclusion;
use paddler_balancer::agent_relay_step::AgentRelayStep;
use paddler_balancer::agent_streaming_request::AgentStreamingRequest;
use paddler_balancer::capability_availability::CapabilityAvailability;
use paddler_balancer::compatibility::compatibility_app_data::CompatibilityAppData;
use paddler_error_chain::error_chain::ErrorChain;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::system_one_response_layout::SystemOneResponseLayout;
use paddler_typesafe_translation::translated_system_one_request::TranslatedSystemOneRequest;

use crate::constants::TYPESAFE_API_PATH_SYSTEM_ONE;
use crate::decision_outcome::DecisionOutcome;
use crate::typesafe_decision_failure::TypeSafeDecisionFailure;
use crate::typesafe_error_body::TypeSafeErrorBody;
use crate::typesafe_json_config::typesafe_json_config;

async fn system_one_http_response<TParams>(
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
                        match response_layout.respond(answers, &summary) {
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

async fn respond(
    app_data: web::Data<CompatibilityAppData>,
    system_one_request: web::Json<SystemOneRequest>,
) -> HttpResponse {
    if let CapabilityAvailability::Lacking(cluster_lacks_capability) = app_data
        .agent_controller_pool
        .capability_availability(DecideParams::REQUESTED_CAPABILITY)
    {
        return TypeSafeDecisionFailure::from(cluster_lacks_capability).into_http_response();
    }

    match system_one_request.into_inner().translate() {
        Ok(TranslatedSystemOneRequest {
            decide_params,
            response_layout,
        }) => system_one_http_response(&response_layout, app_data.agent_relay(decide_params)).await,
        Err(translation_error) => TypeSafeErrorBody {
            detail: ErrorChain {
                error: &translation_error,
            }
            .to_string(),
        }
        .into_http_response(StatusCode::UNPROCESSABLE_ENTITY),
    }
}

pub fn post_system_one(cfg: &mut web::ServiceConfig) {
    cfg.app_data(typesafe_json_config())
        .route(TYPESAFE_API_PATH_SYSTEM_ONE, post().to(respond));
}
