use std::time::SystemTime;

use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::post;

use paddler_messaging::agent_inference_settings::AgentInferenceSettings;
use paddler_messaging::agent_text_generation_settings::AgentTextGenerationSettings;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_openai_translation::responses_delivery::ResponsesDelivery;
use paddler_openai_translation::responses_request::ResponsesRequest;
use paddler_openai_translation::translated_responses_request::TranslatedResponsesRequest;

use crate::agent_streaming_request::AgentStreamingRequest as _;
use crate::cluster_serves_another_inference_mode::ClusterServesAnotherInferenceMode;
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
    let sampling_parameters = match app_data
        .balancer_applicable_state_holder
        .get_agent_desired_state()
        .inference_settings
    {
        AgentInferenceSettings::TextGeneration(AgentTextGenerationSettings {
            sampling_parameters,
            ..
        }) => sampling_parameters,
        served_inference_settings @ (AgentInferenceSettings::Decision(_)
        | AgentInferenceSettings::Embeddings(_)) => {
            return OpenAIFailure::ClusterServesAnotherInferenceMode(
                ClusterServesAnotherInferenceMode {
                    requested_inference_mode: ContinueFromConversationHistoryParams::<
                        ValidatedParametersSchema,
                    >::INFERENCE_MODE,
                    served_inference_mode: served_inference_settings.inference_mode(),
                },
            )
            .into_http_response();
        }
    };

    match responses_request
        .into_inner()
        .translate(SystemTime::now(), &sampling_parameters)
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
