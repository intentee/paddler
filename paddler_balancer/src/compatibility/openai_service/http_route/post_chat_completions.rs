use std::sync::Arc;
use std::time::SystemTime;

use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::post;
use actix_web::web;
use nanoid::nanoid;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::validates::Validates;
use parking_lot::Mutex;
use tokio_stream::StreamExt as _;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::chat_completions_sse_response::chat_completions_sse_response;
use crate::compatibility::openai_service::openai_chat_completion_tool::OpenAIChatCompletionTool;
use crate::compatibility::openai_service::openai_completion_request_params::OpenAICompletionRequestParams;
use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::openai_message::OpenAIMessage;
use crate::compatibility::openai_service::openai_non_streaming_response_transformer::OpenAINonStreamingResponseTransformer;
use crate::compatibility::openai_service::openai_non_streaming_state::OpenAINonStreamingState;
use crate::compatibility::openai_service::openai_streaming_response_transformer::OpenAIStreamingResponseTransformer;
use crate::compatibility::openai_service::openai_streaming_state::OpenAIStreamingState;
use crate::compatibility::openai_service::timestamp_from::timestamp_from;
use crate::require_token_generation_enabled::require_token_generation_enabled;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;

#[post("/v1/chat/completions")]
async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAICompletionRequestParams>,
) -> Result<HttpResponse, Error> {
    if require_token_generation_enabled(&app_data.balancer_applicable_state_holder).is_err() {
        return Ok(HttpResponse::NotImplemented()
            .content_type("application/json")
            .body(
                OpenAIError {
                    error_type: "server_error",
                    message: "Chat completions are disabled while the cluster is configured for embeddings"
                        .to_owned(),
                }
                .to_envelope()
                .to_string(),
            ));
    }

    let openai_params = openai_params.into_inner();

    let validated_tools = match openai_params
        .tools
        .into_iter()
        .filter_map(OpenAIChatCompletionTool::into_tool)
        .map(Validates::validate)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(tools) => tools,
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

    let parse_tool_calls = !validated_tools.is_empty();
    let paddler_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(
            openai_params
                .messages
                .iter()
                .map(OpenAIMessage::to_conversation_message)
                .collect(),
        ),
        enable_thinking: true,
        grammar: None,
        max_tokens: openai_params.max_completion_tokens.unwrap_or(2000),
        parse_tool_calls,
        tools: validated_tools,
    };

    let created =
        timestamp_from(SystemTime::now()).map_err(actix_web::error::ErrorInternalServerError)?;

    if openai_params.stream.unwrap_or(false) {
        let include_usage = openai_params
            .stream_options
            .as_ref()
            .is_some_and(|options| options.include_usage);

        Ok(chat_completions_sse_response(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAIStreamingResponseTransformer {
                created,
                include_usage,
                model: openai_params.model.clone(),
                state: Arc::new(Mutex::new(OpenAIStreamingState::default())),
                system_fingerprint: nanoid!(),
            },
            app_data.shutdown.clone(),
            app_data.drain_counter.clone(),
        ))
    } else {
        let results: Vec<TransformResult> = unbounded_stream_from_agent(
            app_data.buffered_request_manager.clone(),
            app_data.inference_service_configuration.clone(),
            paddler_params,
            OpenAINonStreamingResponseTransformer {
                created,
                model: openai_params.model.clone(),
                state: Arc::new(Mutex::new(OpenAINonStreamingState::default())),
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
                            message: "no completion produced".to_owned(),
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
    use anyhow::Result;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::inference_parameters::InferenceParameters;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
    use serde_json::Value;
    use serde_json::json;
    use tokio_util::sync::CancellationToken;

    use super::AppData;
    use super::register;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::awaitable_counter::AwaitableCounter;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;

    fn app_data_without_agents(max_buffered_requests: i32) -> AppData {
        AppData {
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::ZERO,
                max_buffered_requests,
            )),
            drain_counter: Arc::new(AwaitableCounter::default()),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                cors_allowed_hosts: Vec::new(),
                inference_item_timeout: Duration::ZERO,
            },
            shutdown: CancellationToken::new(),
        }
    }

    fn app_data_with_embeddings_enabled() -> AppData {
        let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::default());

        balancer_applicable_state_holder.set_balancer_applicable_state(Some(
            BalancerApplicableState {
                agent_desired_state: AgentDesiredState {
                    chat_template_override: None,
                    inference_parameters: InferenceParameters {
                        enable_embeddings: true,
                        ..InferenceParameters::default()
                    },
                    model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
                    multimodal_projection: AgentDesiredModel::None,
                },
            },
        ));

        AppData {
            balancer_applicable_state_holder,
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::ZERO,
                0,
            )),
            drain_counter: Arc::new(AwaitableCounter::default()),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                cors_allowed_hosts: Vec::new(),
                inference_item_timeout: Duration::ZERO,
            },
            shutdown: CancellationToken::new(),
        }
    }

    #[actix_web::test]
    async fn rejects_chat_completion_when_embeddings_are_enabled() -> Result<()> {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_with_embeddings_enabled()))
                .configure(register),
        )
        .await;

        let request = TestRequest::post()
            .uri("/v1/chat/completions")
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        let body = read_body(response).await;
        let envelope: Value = serde_json::from_slice(&body)?;

        OpenAIValidator::new()?.validate_error_response(&envelope)?;

        Ok(())
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
            .uri("/v1/chat/completions")
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}],
                "tools": [
                    {
                        "type": "function",
                        "function": {
                            "name": "broken",
                            "description": "tool with an unsatisfiable required field",
                            "parameters": {
                                "type": "object",
                                "properties": {"present": {"type": "string"}},
                                "required": ["absent"]
                            }
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
    async fn opencode_style_tools_are_accepted() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(register),
        )
        .await;

        let request = TestRequest::post()
            .uri("/v1/chat/completions")
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}],
                "tools": [
                    {
                        "type": "function",
                        "function": {
                            "name": "glob",
                            "description": "Fast file pattern matching tool",
                            "parameters": {
                                "$schema": "https://json-schema.org/draft/2020-12/schema",
                                "type": "object",
                                "properties": {
                                    "pattern": {"type": "string", "description": "The glob pattern"},
                                    "path": {"type": "string", "description": "The directory to search in"}
                                },
                                "required": ["pattern"]
                            }
                        }
                    }
                ]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = read_body(response).await;
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(parsed["error"]["type"], "server_error");
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
            .uri("/v1/chat/completions")
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = read_body(response).await;
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(parsed["error"]["type"], "server_error");
        assert!(
            parsed["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Buffered requests overflow")
        );
    }
}
