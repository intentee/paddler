use std::time::SystemTime;

use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::post;

use paddler_balancer::agent_streaming_request::AgentStreamingRequest as _;
use paddler_balancer::capability_availability::CapabilityAvailability;
use paddler_balancer::compatibility::compatibility_app_data::CompatibilityAppData;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_openai_translation::responses_delivery::ResponsesDelivery;
use paddler_openai_translation::responses_request::ResponsesRequest;
use paddler_openai_translation::translated_responses_request::TranslatedResponsesRequest;

use crate::constants::OPENAI_API_PATH_RESPONSES;
use crate::generation_relay::GenerationRelay;
use crate::openai_failure::OpenAIFailure;
use crate::openai_json_config::openai_json_config;
use crate::responses_sse_response::responses_sse_response;

async fn respond(
    app_data: web::Data<CompatibilityAppData>,
    responses_request: web::Json<ResponsesRequest>,
) -> HttpResponse {
    if let CapabilityAvailability::Lacking(cluster_lacks_capability) =
        app_data.agent_controller_pool.capability_availability(
        ContinueFromConversationHistoryParams::<ValidatedParametersSchema>::REQUESTED_CAPABILITY,
    ) {
        return OpenAIFailure::ClusterLacksCapability(cluster_lacks_capability)
            .into_http_response();
    }

    let balancer_applicable_state = app_data
        .balancer_applicable_state_holder
        .get_balancer_applicable_state();

    match responses_request.into_inner().translate(
        SystemTime::now(),
        &balancer_applicable_state
            .balancer_desired_state
            .text_generation
            .sampling_parameters,
    ) {
        Ok(TranslatedResponsesRequest {
            conversation_history_params,
            delivery,
        }) => {
            let generation_relay = GenerationRelay {
                agent_relay: app_data.agent_relay(conversation_history_params),
            };

            match delivery {
                ResponsesDelivery::Buffered(responses_response_header) => {
                    generation_relay
                        .into_http_response(&responses_response_header)
                        .await
                }
                ResponsesDelivery::Streamed(responses_stream) => responses_sse_response(
                    responses_stream,
                    generation_relay.into_progress_events(),
                ),
            }
        }
        Err(translation_error) => {
            OpenAIFailure::RequestUntranslatable(translation_error).into_http_response()
        }
    }
}

pub fn post_responses(cfg: &mut web::ServiceConfig) {
    cfg.app_data(openai_json_config())
        .route(OPENAI_API_PATH_RESPONSES, post().to(respond));
}
