use std::time::SystemTime;

use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::post;

use paddler_openai_translation::chat_completion_delivery::ChatCompletionDelivery;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_openai_translation::translated_chat_completion_request::TranslatedChatCompletionRequest;

use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::openai_service::chat_completions_sse_response::chat_completions_sse_response;
use crate::compatibility::openai_service::generation_relay::GenerationRelay;
use crate::compatibility::openai_service::openai_failure::OpenAIFailure;
use crate::compatibility::openai_service::openai_json_config::openai_json_config;
use crate::constants::OPENAI_API_PATH_CHAT_COMPLETIONS;

async fn respond(
    app_data: web::Data<CompatibilityAppData>,
    chat_completion_request: web::Json<ChatCompletionRequest>,
) -> HttpResponse {
    match chat_completion_request
        .into_inner()
        .translate(SystemTime::now())
    {
        Ok(TranslatedChatCompletionRequest {
            conversation_history_params,
            delivery,
        }) => {
            let generation_relay = GenerationRelay {
                agent_relay: app_data.agent_relay(conversation_history_params),
            };

            match delivery {
                ChatCompletionDelivery::Buffered(chat_completion_header) => {
                    generation_relay
                        .into_http_response(&chat_completion_header)
                        .await
                }
                ChatCompletionDelivery::Streamed(chat_completion_stream) => {
                    chat_completions_sse_response(
                        chat_completion_stream,
                        generation_relay.into_progress_events(),
                    )
                }
            }
        }
        Err(translation_error) => {
            OpenAIFailure::RequestUntranslatable(translation_error).into_http_response()
        }
    }
}

pub fn post_chat_completions(cfg: &mut web::ServiceConfig) {
    cfg.app_data(openai_json_config())
        .route(OPENAI_API_PATH_CHAT_COMPLETIONS, post().to(respond));
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
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::json;
    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;

    use super::CompatibilityAppData;
    use super::post_chat_completions;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::constants::OPENAI_API_PATH_CHAT_COMPLETIONS;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use crate::resolved_socket_addr::ResolvedSocketAddr;

    fn app_data_without_agents(max_buffered_requests: u64) -> CompatibilityAppData {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());

        CompatibilityAppData {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::new(
                watch::channel(Arc::new(BalancerApplicableState::initial(
                    BalancerDesiredState::default(),
                )))
                .1,
            )),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                agent_controller_pool,
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

    #[actix_web::test]
    async fn invalid_tool_schema_returns_bad_request() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(post_chat_completions),
        )
        .await;

        let request = TestRequest::post()
            .uri(OPENAI_API_PATH_CHAT_COMPLETIONS)
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
    async fn rejects_a_tool_that_is_not_a_function_with_an_openai_error() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(post_chat_completions),
        )
        .await;

        let request = TestRequest::post()
            .uri(OPENAI_API_PATH_CHAT_COMPLETIONS)
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}],
                "tools": [{"type": "web_search"}]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = read_body(response).await;
        let envelope: Value = from_slice(&body).expect("the error response must be JSON");

        OpenAIValidator::new()
            .expect("the OpenAI validator must load its schemas")
            .validate_error_response(&envelope)
            .expect("the error response must conform to the OpenAI schema");
        assert_eq!(
            envelope.pointer("/error/type"),
            Some(&json!("invalid_request_error"))
        );
    }

    #[actix_web::test]
    async fn non_streaming_request_overflowing_the_buffer_returns_service_unavailable() {
        let app = init_service(
            App::new()
                .app_data(Data::new(app_data_without_agents(0)))
                .configure(post_chat_completions),
        )
        .await;

        let request = TestRequest::post()
            .uri(OPENAI_API_PATH_CHAT_COMPLETIONS)
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .to_request();

        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

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
