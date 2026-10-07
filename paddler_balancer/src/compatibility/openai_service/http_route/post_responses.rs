use std::time::SystemTime;

use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::post;

use paddler_openai_translation::responses_delivery::ResponsesDelivery;
use paddler_openai_translation::responses_request::ResponsesRequest;
use paddler_openai_translation::translated_responses_request::TranslatedResponsesRequest;

use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use crate::compatibility::openai_service::openai_failure::OpenAIFailure;
use crate::compatibility::openai_service::openai_http_response::openai_http_response;
use crate::compatibility::openai_service::openai_json_config::openai_json_config;
use crate::compatibility::openai_service::responses_sse_response::responses_sse_response;

async fn respond(
    app_data: web::Data<CompatibilityAppData>,
    responses_request: web::Json<ResponsesRequest>,
) -> HttpResponse {
    let agent_desired_state = app_data
        .balancer_applicable_state_holder
        .get_agent_desired_state();

    match responses_request
        .into_inner()
        .translate(SystemTime::now(), &agent_desired_state.inference_settings)
    {
        Ok(TranslatedResponsesRequest {
            conversation_history_params,
            delivery,
        }) => {
            let generated_tokens = app_data.agent_result_stream(conversation_history_params);

            match delivery {
                ResponsesDelivery::Buffered(responses_response_header) => {
                    openai_http_response(&responses_response_header, generated_tokens).await
                }
                ResponsesDelivery::Streamed(responses_stream) => {
                    responses_sse_response(responses_stream, generated_tokens)
                }
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
