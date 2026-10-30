use std::time::SystemTime;

use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::post;

use paddler_openai_translation::responses_delivery::ResponsesDelivery;
use paddler_openai_translation::responses_request::ResponsesRequest;
use paddler_openai_translation::translated_responses_request::TranslatedResponsesRequest;

use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::openai_service::generation_progress_events::generation_progress_events;
use crate::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use crate::compatibility::openai_service::openai_failure::OpenAIFailure;
use crate::compatibility::openai_service::openai_http_response::openai_http_response;
use crate::compatibility::openai_service::openai_json_config::openai_json_config;
use crate::compatibility::openai_service::responses_sse_response::responses_sse_response;

async fn respond(
    app_data: web::Data<CompatibilityAppData>,
    responses_request: web::Json<ResponsesRequest>,
) -> HttpResponse {
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
            let agent_relay = app_data.agent_relay(conversation_history_params);

            match delivery {
                ResponsesDelivery::Buffered(responses_response_header) => {
                    openai_http_response(&responses_response_header, agent_relay).await
                }
                ResponsesDelivery::Streamed(responses_stream) => responses_sse_response(
                    responses_stream,
                    generation_progress_events(agent_relay),
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
        .route(OpenAIApiPath::RESPONSES, post().to(respond));
}
