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

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::validates::Validates;

use crate::cluster_token_generation_mode::ClusterTokenGenerationMode;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::chat_completions_sse_response::chat_completions_sse_response;
use crate::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use crate::compatibility::openai_service::openai_chat_completion_tool::OpenAIChatCompletionTool;
use crate::compatibility::openai_service::openai_completion_request_params::OpenAICompletionRequestParams;
use crate::compatibility::openai_service::openai_default_max_tokens::OPENAI_DEFAULT_MAX_TOKENS;
use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;
use crate::compatibility::openai_service::openai_json_config::openai_json_config;
use crate::compatibility::openai_service::openai_json_response::openai_json_response;
use crate::compatibility::openai_service::openai_message::OpenAIMessage;
use crate::compatibility::openai_service::openai_non_streaming_response_transformer::OpenAINonStreamingResponseTransformer;
use crate::compatibility::openai_service::openai_non_streaming_state::OpenAINonStreamingState;
use crate::compatibility::openai_service::openai_streaming_response_transformer::OpenAIStreamingResponseTransformer;
use crate::compatibility::openai_service::openai_streaming_state::OpenAIStreamingState;
use crate::compatibility::openai_service::timestamp_from::timestamp_from;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;
use crate::unbounded_stream_from_agent_params::UnboundedStreamFromAgentParams;

async fn respond(
    app_data: web::Data<AppData>,
    openai_params: web::Json<OpenAICompletionRequestParams>,
) -> Result<HttpResponse, Error> {
    if app_data
        .balancer_applicable_state_holder
        .token_generation_mode()
        == ClusterTokenGenerationMode::DisabledForEmbeddings
    {
        return Ok(OpenAIError {
            error_type: OpenAIErrorType::ServerError,
            message: "Chat completions are disabled while the cluster is configured for embeddings"
                .to_owned(),
        }
        .to_http_response(StatusCode::NOT_IMPLEMENTED));
    }

    let openai_params = openai_params.into_inner();
    let enable_thinking = openai_params.enables_thinking();
    let max_tokens = openai_params
        .requested_max_tokens()
        .unwrap_or(OPENAI_DEFAULT_MAX_TOKENS);

    let validated_tools = match openai_params
        .tools
        .into_iter()
        .map(OpenAIChatCompletionTool::into_tool)
        .map(Validates::validate)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(tools) => tools,
        Err(err) => {
            return Ok(OpenAIError {
                error_type: OpenAIErrorType::InvalidRequestError,
                message: err.to_string(),
            }
            .to_http_response(StatusCode::BAD_REQUEST));
        }
    };

    let parse_tool_calls = !validated_tools.is_empty();
    let paddler_params = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(
            openai_params
                .messages
                .into_iter()
                .map(OpenAIMessage::into_conversation_message)
                .collect(),
        ),
        enable_thinking,
        grammar: None,
        max_tokens,
        parse_tool_calls,
        tools: validated_tools,
    };

    let created = timestamp_from(SystemTime::now()).map_err(ErrorInternalServerError)?;

    if openai_params.stream.unwrap_or(false) {
        let include_usage = openai_params
            .stream_options
            .as_ref()
            .is_some_and(|options| options.include_usage);

        Ok(chat_completions_sse_response(unbounded_stream_from_agent(
            UnboundedStreamFromAgentParams {
                buffered_request_manager: app_data.buffered_request_manager.clone(),
                inference_service_configuration: app_data.inference_service_configuration.clone(),
                request_params: paddler_params,
                shutdown: app_data.shutdown.clone(),
                transformer: OpenAIStreamingResponseTransformer {
                    created,
                    include_usage,
                    model: openai_params.model.clone(),
                    state: Arc::new(Mutex::new(OpenAIStreamingState::default())),
                    system_fingerprint: nanoid!(),
                },
            },
        )))
    } else {
        Ok(openai_json_response(
            unbounded_stream_from_agent(UnboundedStreamFromAgentParams {
                buffered_request_manager: app_data.buffered_request_manager.clone(),
                inference_service_configuration: app_data.inference_service_configuration.clone(),
                request_params: paddler_params,
                shutdown: app_data.shutdown.clone(),
                transformer: OpenAINonStreamingResponseTransformer {
                    created,
                    model: openai_params.model.clone(),
                    state: Arc::new(Mutex::new(OpenAINonStreamingState::default())),
                },
            }),
            "no completion produced",
        )
        .await)
    }
}

pub fn post_chat_completions(cfg: &mut web::ServiceConfig) {
    cfg.app_data(openai_json_config())
        .route(OpenAIApiPath::CHAT_COMPLETIONS, post().to(respond));
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
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::json;
    use tokio_util::sync::CancellationToken;

    use paddler_inference_parameters::inference_parameters::InferenceParameters;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;

    use super::AppData;
    use super::post_chat_completions;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::compatibility::openai_service::openai_api_path::OpenAIApiPath;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use crate::resolved_socket_addr::ResolvedSocketAddr;

    fn app_data_without_agents(max_buffered_requests: u64) -> AppData {
        AppData {
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::new(
                BalancerApplicableState::from(BalancerDesiredState::default()),
            )),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::ZERO,
                max_buffered_requests,
            )),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: ResolvedSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))),
                cors_allowed_hosts: Vec::new(),
                inference_item_timeout: Duration::ZERO,
            },
            shutdown: CancellationToken::new(),
        }
    }

    fn app_data_with_embeddings_enabled() -> AppData {
        let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::new(
            BalancerApplicableState::from(BalancerDesiredState::default()),
        ));

        balancer_applicable_state_holder.set_balancer_applicable_state(BalancerApplicableState {
            agent_desired_state: AgentDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    enable_embeddings: true,
                    ..InferenceParameters::default()
                },
                model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
                multimodal_projection: AgentDesiredModel::None,
            },
        });

        AppData {
            balancer_applicable_state_holder,
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::ZERO,
                0,
            )),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: ResolvedSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))),
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
                .configure(post_chat_completions),
        )
        .await;

        let request = TestRequest::post()
            .uri(OpenAIApiPath::CHAT_COMPLETIONS)
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        let body = read_body(response).await;
        let envelope: Value = from_slice(&body)?;

        OpenAIValidator::new()?.validate_error_response(&envelope)?;

        Ok(())
    }

    #[actix_web::test]
    async fn invalid_tool_schema_returns_bad_request() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(post_chat_completions),
        )
        .await;

        let request = TestRequest::post()
            .uri(OpenAIApiPath::CHAT_COMPLETIONS)
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
        let parsed: Value = from_slice(&body).unwrap();

        assert_eq!(
            parsed,
            json!({
                "error": {
                    "message": "Required field 'absent' not found in properties",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": null
                }
            })
        );
    }

    #[actix_web::test]
    async fn rejects_a_tool_that_is_not_a_function_with_an_openai_error() -> Result<()> {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(post_chat_completions),
        )
        .await;

        let request = TestRequest::post()
            .uri(OpenAIApiPath::CHAT_COMPLETIONS)
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}],
                "tools": [{"type": "web_search"}]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = read_body(response).await;
        let envelope: Value = from_slice(&body)?;

        OpenAIValidator::new()?.validate_error_response(&envelope)?;
        assert_eq!(envelope["error"]["type"], "invalid_request_error");

        Ok(())
    }

    #[actix_web::test]
    async fn opencode_style_tools_are_accepted() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(post_chat_completions),
        )
        .await;

        let request = TestRequest::post()
            .uri(OpenAIApiPath::CHAT_COMPLETIONS)
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
        let parsed: Value = from_slice(&body).unwrap();

        assert_eq!(parsed["error"]["type"], "server_error");
    }

    #[actix_web::test]
    async fn non_streaming_request_without_available_agent_returns_internal_server_error() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(post_chat_completions),
        )
        .await;

        let request = TestRequest::post()
            .uri(OpenAIApiPath::CHAT_COMPLETIONS)
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = read_body(response).await;
        let parsed: Value = from_slice(&body).unwrap();

        assert_eq!(
            parsed,
            json!({
                "error": {
                    "message": "Buffered requests overflow",
                    "type": "server_error",
                    "param": null,
                    "code": null
                }
            })
        );
    }
}
