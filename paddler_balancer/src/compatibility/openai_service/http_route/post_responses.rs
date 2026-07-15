use std::sync::Arc;
use std::time::SystemTime;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use nanoid::nanoid;
use parking_lot::Mutex;
use tokio_stream::StreamExt as _;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::openai_responses_request_params::OpenAIResponsesRequestParams;
use crate::compatibility::openai_service::responses_non_streaming_response_transformer::ResponsesNonStreamingResponseTransformer;
use crate::compatibility::openai_service::responses_non_streaming_state::ResponsesNonStreamingState;
use crate::compatibility::openai_service::responses_response_builder::ResponsesResponseBuilder;
use crate::compatibility::openai_service::responses_streaming_response_transformer::ResponsesStreamingResponseTransformer;
use crate::compatibility::openai_service::responses_streaming_state::ResponsesStreamingState;
use crate::compatibility::openai_service::sse_response_from_agent::sse_response_from_agent;
use crate::compatibility::openai_service::timestamp_from::timestamp_from;
use crate::require_token_generation_enabled::require_token_generation_enabled;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;

#[post("/v1/responses")]
async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAIResponsesRequestParams>,
) -> Result<HttpResponse, Error> {
    if require_token_generation_enabled(&app_data.balancer_applicable_state_holder).is_err() {
        return Ok(HttpResponse::NotImplemented()
            .content_type("application/json")
            .body(
                OpenAIError {
                    error_type: "server_error",
                    message:
                        "Responses are disabled while the cluster is configured for embeddings"
                            .to_owned(),
                }
                .to_envelope()
                .to_string(),
            ));
    }

    let prepared = match openai_params.into_inner().into_prepared() {
        Ok(prepared) => prepared,
        Err(err) => {
            return Ok(HttpResponse::BadRequest()
                .content_type("application/json")
                .body(
                    OpenAIError {
                        error_type: "invalid_request_error",
                        message: err.to_string(),
                    }
                    .to_envelope()
                    .to_string(),
                ));
        }
    };

    let created_at =
        timestamp_from(SystemTime::now()).map_err(actix_web::error::ErrorInternalServerError)?;

    let builder = ResponsesResponseBuilder {
        id: format!("resp_{}", nanoid!()),
        created_at,
        model: prepared.model,
        instructions: prepared.instructions,
    };

    if prepared.stream {
        Ok(sse_response_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            prepared.paddler_params,
            ResponsesStreamingResponseTransformer {
                builder,
                state: Arc::new(Mutex::new(ResponsesStreamingState::default())),
            },
            app_data.shutdown.clone(),
            app_data.drain_counter.clone(),
        ))
    } else {
        let results: Vec<TransformResult> = unbounded_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            prepared.paddler_params,
            ResponsesNonStreamingResponseTransformer {
                builder,
                state: Arc::new(Mutex::new(ResponsesNonStreamingState::default())),
            },
            app_data.shutdown.clone(),
            app_data.drain_counter.clone(),
        )
        .collect()
        .await;

        if let Some(TransformResult::Error(error_json)) = results
            .iter()
            .find(|result| matches!(result, TransformResult::Error(_)))
        {
            return Ok(HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(error_json.clone()));
        }

        let body = results.into_iter().find_map(|result| match result {
            TransformResult::Chunk(content) => Some(content),
            TransformResult::Discard | TransformResult::Error(_) => None,
        });

        Ok(body.map_or_else(
            || {
                HttpResponse::InternalServerError()
                    .content_type("application/json")
                    .body(
                        OpenAIError {
                            error_type: "server_error",
                            message: "no response produced".to_owned(),
                        }
                        .to_envelope()
                        .to_string(),
                    )
            },
            |json_body| {
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(json_body)
            },
        ))
    }
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}
