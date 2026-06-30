use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use nanoid::nanoid;
use parking_lot::Mutex;
use tokio_stream::StreamExt as _;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::current_unix_timestamp::current_unix_timestamp;
use crate::compatibility::openai_service::non_streaming_http_response::non_streaming_http_response;
use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::openai_responses_request_params::OpenAIResponsesRequestParams;
use crate::compatibility::openai_service::responses_non_streaming_response_transformer::ResponsesNonStreamingResponseTransformer;
use crate::compatibility::openai_service::responses_non_streaming_state::ResponsesNonStreamingState;
use crate::compatibility::openai_service::responses_response_builder::ResponsesResponseBuilder;
use crate::compatibility::openai_service::responses_streaming_response_transformer::ResponsesStreamingResponseTransformer;
use crate::compatibility::openai_service::responses_streaming_state::ResponsesStreamingState;
use crate::compatibility::openai_service::sse_response_from_agent::sse_response_from_agent;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;

#[post("/v1/responses")]
async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAIResponsesRequestParams>,
) -> Result<HttpResponse, Error> {
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

    let created_at = current_unix_timestamp();

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
        )
        .collect()
        .await;

        Ok(non_streaming_http_response(results, "no response produced"))
    }
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::test::call_service;
    use actix_web::test::init_service;
    use actix_web::test::read_body;
    use actix_web::web::Data;
    use serde_json::json;
    use tokio_util::sync::CancellationToken;

    use super::AppData;
    use super::register;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;

    fn app_data_without_agents(max_buffered_requests: i32) -> AppData {
        AppData {
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::ZERO,
                max_buffered_requests,
            )),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                cors_allowed_hosts: Vec::new(),
                inference_item_timeout: Duration::ZERO,
            },
            shutdown: CancellationToken::new(),
        }
    }

    #[actix_web::test]
    async fn invalid_tool_schema_returns_bad_request() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(register),
        )
        .await;

        let request = TestRequest::post()
            .uri("/v1/responses")
            .set_json(json!({
                "model": "test-model",
                "input": "hi",
                "tools": [
                    {
                        "type": "function",
                        "name": "broken",
                        "parameters": {
                            "type": "object",
                            "properties": {"present": {"type": "string"}},
                            "required": ["absent"]
                        }
                    }
                ]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = read_body(response).await;
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(parsed["error"]["type"], "invalid_request_error");
        assert!(
            parsed["error"]["message"]
                .as_str()
                .unwrap()
                .contains("absent")
        );
    }

    #[actix_web::test]
    async fn non_streaming_request_without_available_agent_returns_internal_server_error() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(register),
        )
        .await;

        let request = TestRequest::post()
            .uri("/v1/responses")
            .set_json(json!({
                "model": "test-model",
                "input": "hi"
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = read_body(response).await;
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(parsed.get("error").is_some());
    }
}
