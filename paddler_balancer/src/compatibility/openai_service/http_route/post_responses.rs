use std::sync::Arc;
use std::time::SystemTime;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::error::ErrorInternalServerError;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::post;
use nanoid::nanoid;
use parking_lot::Mutex;

use paddler_inference_parameters::inference_parameters::InferenceParameters;

use crate::cluster_token_generation_mode::ClusterTokenGenerationMode;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;
use crate::compatibility::openai_service::openai_json_config::openai_json_config;
use crate::compatibility::openai_service::openai_json_response::openai_json_response;
use crate::compatibility::openai_service::openai_responses_request_params::OpenAIResponsesRequestParams;
use crate::compatibility::openai_service::responses_non_streaming_response_transformer::ResponsesNonStreamingResponseTransformer;
use crate::compatibility::openai_service::responses_non_streaming_state::ResponsesNonStreamingState;
use crate::compatibility::openai_service::responses_response_header::ResponsesResponseHeader;
use crate::compatibility::openai_service::responses_sse_response::responses_sse_response;
use crate::compatibility::openai_service::responses_streaming_response_transformer::ResponsesStreamingResponseTransformer;
use crate::compatibility::openai_service::responses_streaming_state::ResponsesStreamingState;
use crate::compatibility::openai_service::timestamp_from::timestamp_from;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;
use crate::unbounded_stream_from_agent_params::UnboundedStreamFromAgentParams;

async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAIResponsesRequestParams>,
) -> Result<HttpResponse, Error> {
    if app_data
        .balancer_applicable_state_holder
        .token_generation_mode()
        == ClusterTokenGenerationMode::DisabledForEmbeddings
    {
        return Ok(OpenAIError {
            error_type: OpenAIErrorType::ServerError,
            message: "Responses are disabled while the cluster is configured for embeddings"
                .to_owned(),
        }
        .to_http_response(StatusCode::NOT_IMPLEMENTED));
    }

    let prepared = match openai_params.into_inner().into_prepared() {
        Ok(prepared) => prepared,
        Err(err) => {
            return Ok(OpenAIError {
                error_type: OpenAIErrorType::InvalidRequestError,
                message: err.to_string(),
            }
            .to_http_response(StatusCode::BAD_REQUEST));
        }
    };

    let created_at = timestamp_from(SystemTime::now()).map_err(ErrorInternalServerError)?;

    let InferenceParameters {
        temperature, top_p, ..
    } = app_data
        .balancer_applicable_state_holder
        .get_agent_desired_state()
        .inference_parameters;
    let header = ResponsesResponseHeader {
        id: format!("resp_{}", nanoid!()),
        created_at,
        model: prepared.model,
        instructions: prepared.instructions,
        temperature,
        top_p,
    };

    if prepared.stream {
        Ok(responses_sse_response(unbounded_stream_from_agent(
            UnboundedStreamFromAgentParams {
                buffered_request_manager: app_data.buffered_request_manager.clone(),
                inference_service_configuration: app_data.inference_service_configuration.clone(),
                request_params: prepared.paddler_params,
                shutdown: app_data.shutdown.clone(),
                transformer: ResponsesStreamingResponseTransformer {
                    header,
                    state: Arc::new(Mutex::new(ResponsesStreamingState::default())),
                },
            },
        )))
    } else {
        Ok(openai_json_response(
            unbounded_stream_from_agent(UnboundedStreamFromAgentParams {
                buffered_request_manager: app_data.buffered_request_manager.clone(),
                inference_service_configuration: app_data.inference_service_configuration.clone(),
                request_params: prepared.paddler_params,
                shutdown: app_data.shutdown.clone(),
                transformer: ResponsesNonStreamingResponseTransformer {
                    header,
                    state: Arc::new(Mutex::new(ResponsesNonStreamingState::default())),
                },
            }),
            "no response produced",
        )
        .await)
    }
}

pub fn post_responses(cfg: &mut web::ServiceConfig) {
    cfg.app_data(openai_json_config())
        .route(OpenAIApiPath::RESPONSES, post().to(respond));
}
