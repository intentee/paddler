mod inference_socket_controller_context;
mod spawn_token_generation_mode_watcher;

use std::fmt::Debug;
use std::sync::Arc;

use actix_web::rt;
use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::get;
use actix_web::web::Data;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use actix_ws::Session;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::streamable_result::StreamableResult;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::validates::Validates as _;
use tokio_util::sync::CancellationToken;

use self::inference_socket_controller_context::InferenceSocketControllerContext;
use self::spawn_token_generation_mode_watcher::spawn_token_generation_mode_watcher;
use crate::agent_controller::AgentController;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::cluster_token_generation_mode::ClusterTokenGenerationMode;
use crate::continuation_decision::ContinuationDecision;
use crate::controls_session::ControlsSession as _;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::inference_service::app_data::AppData;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::manages_senders::ManagesSenders;
use crate::request_cancellation_token_guard::RequestCancellationTokenGuard;
use crate::request_cancellation_tokens::RequestCancellationTokens;
use crate::request_from_agent::request_from_agent;
use crate::websocket_session_controller::WebSocketSessionController;

type InferenceJsonRpcMessage = InferenceServerMessage<RawParametersSchema>;
type InferenceJsonRpcRequest = InferenceServerRequest<RawParametersSchema>;

const TOKEN_GENERATION_DISABLED_MESSAGE: &str =
    "Token generation is disabled while the cluster is configured for embeddings";

async fn send_token_generation_disabled(
    request_id: String,
    websocket_session_controller: &mut WebSocketSessionController<OutgoingMessage>,
) {
    if let Err(err) = websocket_session_controller
        .send_response(OutgoingMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: request_id.clone(),
            response: OutgoingResponse::GeneratedToken(
                GeneratedTokenResult::TokenGenerationDisabled(
                    TOKEN_GENERATION_DISABLED_MESSAGE.to_owned(),
                ),
            ),
        }))
        .await
    {
        error!(
            "Failed to send token-generation-disabled response for request {request_id:?}: {err}"
        );
    }
}

async fn handle_inference_request<TParams>(
    connection_close: &CancellationToken,
    context: Arc<InferenceSocketControllerContext>,
    params: TParams,
    request_id: String,
    mut websocket_session_controller: WebSocketSessionController<OutgoingMessage>,
) where
    TParams: Debug + Into<AgentJsonRpcRequest> + Send + 'static,
    AgentController: HandlesAgentStreamingResponse<TParams>,
    <<AgentController as HandlesAgentStreamingResponse<TParams>>::SenderCollection as ManagesSenders>::Value: Debug + Into<OutgoingResponse> + StreamableResult,
{
    match ClusterTokenGenerationMode::from_applicable_state_holder(
        &context.balancer_applicable_state_holder,
    ) {
        ClusterTokenGenerationMode::DisabledForEmbeddings => {
            send_token_generation_disabled(request_id, &mut websocket_session_controller).await;
        }
        ClusterTokenGenerationMode::Enabled => {
            let request_cancellation_token_guard = RequestCancellationTokenGuard::register(
                connection_close,
                context.request_cancellation_tokens.clone(),
                request_id.clone(),
            );

            rt::spawn(async move {
                request_from_agent(
                    context.buffered_request_manager.clone(),
                    request_cancellation_token_guard.cancellation_token.clone(),
                    context.inference_service_configuration.clone(),
                    params,
                    request_id,
                    websocket_session_controller,
                    context.shutdown.clone(),
                )
                .await;
            });
        }
    }
}

struct InferenceSocketController {
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    buffered_request_manager: Arc<BufferedRequestManager>,
    inference_service_configuration: InferenceServiceConfiguration,
    shutdown: CancellationToken,
}

#[async_trait]
impl ControlsWebSocketEndpoint for InferenceSocketController {
    type Context = InferenceSocketControllerContext;
    type IncomingMessage = InferenceJsonRpcMessage;
    type OutgoingMessage = OutgoingMessage;

    fn create_context(&self) -> Self::Context {
        InferenceSocketControllerContext {
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
            request_cancellation_tokens: Arc::new(RequestCancellationTokens::default()),
            shutdown: self.shutdown.clone(),
        }
    }

    async fn handle_deserialized_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision> {
        match deserialized_message {
            InferenceJsonRpcMessage::Error(ErrorEnvelope {
                request_id,
                error: JsonRpcError { code, description },
            }) => {
                error!(
                    "Received error from client: code: {code}, description: {description:?}, request_id: {request_id:?}"
                );
            }
            InferenceJsonRpcMessage::Notification(
                InferenceServerNotification::StopRespondingTo(request_id),
            ) => {
                context.request_cancellation_tokens.cancel(&request_id);
            }
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: request_id,
                request:
                    InferenceJsonRpcRequest::ContinueFromConversationHistory(
                        conversation_history_params,
                    ),
            }) => {
                let validated_params = conversation_history_params.validate()?;

                handle_inference_request(
                    &connection_close,
                    context,
                    validated_params,
                    request_id,
                    websocket_session_controller,
                )
                .await;
            }
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: request_id,
                request: InferenceJsonRpcRequest::ContinueFromRawPrompt(raw_prompt_params),
            }) => {
                handle_inference_request(
                    &connection_close,
                    context,
                    raw_prompt_params,
                    request_id,
                    websocket_session_controller,
                )
                .await;
            }
        }

        Ok(ContinuationDecision::Continue)
    }

    async fn on_connection_start(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        session: &mut Session,
    ) -> Result<ContinuationDecision> {
        spawn_token_generation_mode_watcher(
            context.balancer_applicable_state_holder.clone(),
            connection_close,
            session.clone(),
        );

        Ok(ContinuationDecision::Continue)
    }
}

#[get("/api/v1/inference_socket")]
async fn respond(
    app_data: Data<AppData>,
    payload: Payload,
    http_request: HttpRequest,
) -> Result<HttpResponse, Error> {
    let inference_socket_controller = InferenceSocketController {
        balancer_applicable_state_holder: app_data.balancer_applicable_state_holder.clone(),
        buffered_request_manager: app_data.buffered_request_manager.clone(),
        inference_service_configuration: app_data.inference_service_configuration.clone(),
        shutdown: app_data.shutdown.clone(),
    };

    inference_socket_controller.respond(payload, http_request, app_data.shutdown.clone())
}

pub fn register(service_config: &mut ServiceConfig) {
    service_config.service(respond);
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::mem::discriminant;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::FromRequest as _;
    use actix_web::http::StatusCode;
    use actix_web::http::header;
    use actix_web::test::TestRequest;
    use actix_web::test::call_service;
    use actix_web::test::init_service;
    use actix_web::web::Data;
    use actix_web::web::Payload;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use crate::agent_controller::AgentController;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::awaitable_counter::AwaitableCounter;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::continuation_decision::ContinuationDecision;
    use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint as _;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::websocket_session_controller::WebSocketSessionController;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::atomic_value::AtomicValue;
    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::inference_parameters::InferenceParameters;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
    use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
    use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
    use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

    use super::AppData;
    use super::InferenceJsonRpcMessage;
    use super::InferenceJsonRpcRequest;
    use super::InferenceServiceConfiguration;
    use super::InferenceSocketController;
    use super::InferenceSocketControllerContext;
    use super::OutgoingMessage;
    use super::RequestCancellationTokens;
    use super::register;

    struct RegisteredAgent {
        pool: Arc<AgentControllerPool>,
        agent_message_rx: mpsc::UnboundedReceiver<AgentJsonRpcMessage>,
    }

    fn pool_with_one_free_slot(agent_id: &str) -> RegisteredAgent {
        let pool = Arc::new(AgentControllerPool::default());
        let (agent_message_tx, agent_message_rx) = mpsc::unbounded_channel();
        let agent_controller = Arc::new(AgentController {
            agent_message_tx,
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            connection_close: CancellationToken::new(),
            desired_slots_total: AtomicValue::<AtomicI32>::new(1),
            download_current: AtomicValue::<AtomicU64>::new(0),
            download_filename: RwLock::new(None),
            download_indeterminate: AtomicValue::<AtomicBool>::new(true),
            download_total: AtomicValue::<AtomicU64>::new(0),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            id: agent_id.to_owned(),
            issues: RwLock::new(BTreeSet::new()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            model_path: RwLock::new(None),
            name: None,
            newest_update_version: AtomicValue::<AtomicI32>::new(0),
            slots_processing: AtomicValue::<AtomicI32>::new(0),
            slots_total: AtomicValue::<AtomicI32>::new(1),
            state_application_status_code: AtomicValue::<AtomicI32>::new(
                AgentStateApplicationStatus::Fresh as i32,
            ),
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
        });

        pool.register_agent_controller(agent_id.to_owned(), agent_controller)
            .unwrap();

        RegisteredAgent {
            pool,
            agent_message_rx,
        }
    }

    fn context_with_pool(
        pool: Arc<AgentControllerPool>,
        balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    ) -> Arc<InferenceSocketControllerContext> {
        Arc::new(InferenceSocketControllerContext {
            balancer_applicable_state_holder,
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                pool,
                Duration::from_mins(1),
                10,
            )),
            inference_service_configuration: inference_service_configuration(),
            request_cancellation_tokens: Arc::new(RequestCancellationTokens::default()),
            shutdown: CancellationToken::new(),
        })
    }

    fn embeddings_enabled_holder() -> Arc<BalancerApplicableStateHolder> {
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

        balancer_applicable_state_holder
    }

    async fn open_session_controller() -> WebSocketSessionController<OutgoingMessage> {
        let (request, mut raw_payload) = TestRequest::get()
            .insert_header((header::CONNECTION, "upgrade"))
            .insert_header((header::UPGRADE, "websocket"))
            .insert_header((header::SEC_WEBSOCKET_VERSION, "13"))
            .insert_header((header::SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_http_parts();
        let payload = Payload::from_request(&request, &mut raw_payload)
            .await
            .unwrap();
        let (_response, session, _msg_stream) = actix_ws::handle(&request, payload).unwrap();

        WebSocketSessionController::new(session)
    }

    fn inference_service_configuration() -> InferenceServiceConfiguration {
        InferenceServiceConfiguration {
            addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            cors_allowed_hosts: vec!["http://localhost".to_owned()],
            inference_item_timeout: Duration::from_secs(30),
        }
    }

    #[actix_web::test]
    async fn create_context_copies_controller_state() {
        let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::default());
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            Arc::new(AgentControllerPool::default()),
            Duration::from_mins(1),
            10,
        ));
        let shutdown = CancellationToken::new();
        let controller = InferenceSocketController {
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            inference_service_configuration: inference_service_configuration(),
            shutdown: shutdown.clone(),
        };

        let context = controller.create_context();

        assert!(Arc::ptr_eq(
            &context.balancer_applicable_state_holder,
            &balancer_applicable_state_holder
        ));
        assert!(Arc::ptr_eq(
            &context.buffered_request_manager,
            &buffered_request_manager
        ));
        assert_eq!(
            context
                .inference_service_configuration
                .inference_item_timeout,
            Duration::from_secs(30)
        );
        assert_eq!(
            context.inference_service_configuration.cors_allowed_hosts,
            vec!["http://localhost".to_owned()]
        );

        shutdown.cancel();

        assert!(context.shutdown.is_cancelled());
    }

    #[actix_web::test]
    async fn respond_upgrades_websocket_handshake() {
        let app_data = Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::from_mins(1),
                10,
            )),
            drain_counter: Arc::new(AwaitableCounter::default()),
            inference_service_configuration: inference_service_configuration(),
            shutdown: CancellationToken::new(),
        });
        let app = init_service(App::new().app_data(app_data).configure(register)).await;

        let request = TestRequest::get()
            .uri("/api/v1/inference_socket")
            .insert_header((header::UPGRADE, "websocket"))
            .insert_header((header::CONNECTION, "Upgrade"))
            .insert_header((header::SEC_WEBSOCKET_VERSION, "13"))
            .insert_header((header::SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    }

    #[actix_web::test]
    async fn respond_upgrades_when_embeddings_are_enabled() {
        let app_data = Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder: embeddings_enabled_holder(),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::from_mins(1),
                10,
            )),
            drain_counter: Arc::new(AwaitableCounter::default()),
            inference_service_configuration: inference_service_configuration(),
            shutdown: CancellationToken::new(),
        });
        let app = init_service(App::new().app_data(app_data).configure(register)).await;

        let request = TestRequest::get()
            .uri("/api/v1/inference_socket")
            .insert_header((header::UPGRADE, "websocket"))
            .insert_header((header::CONNECTION, "Upgrade"))
            .insert_header((header::SEC_WEBSOCKET_VERSION, "13"))
            .insert_header((header::SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    }

    #[actix_web::test]
    async fn handle_raw_prompt_request_replies_disabled_without_dispatch_in_embeddings_mode() {
        let RegisteredAgent {
            pool,
            mut agent_message_rx,
        } = pool_with_one_free_slot("agent-raw-prompt-embeddings");
        let session_controller = open_session_controller().await;

        let continuation_decision = InferenceSocketController::handle_deserialized_message(
            CancellationToken::new(),
            context_with_pool(pool, embeddings_enabled_holder()),
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: "request-raw-prompt-embeddings".to_owned(),
                request: InferenceJsonRpcRequest::ContinueFromRawPrompt(
                    ContinueFromRawPromptParams {
                        grammar: None,
                        max_tokens: 1,
                        raw_prompt: "fixture prompt".to_owned(),
                    },
                ),
            }),
            session_controller,
        )
        .await
        .unwrap();

        assert_eq!(
            discriminant(&continuation_decision),
            discriminant(&ContinuationDecision::Continue)
        );
        assert!(agent_message_rx.try_recv().is_err());
    }

    #[actix_web::test]
    async fn handle_conversation_history_request_replies_disabled_without_dispatch_in_embeddings_mode()
     {
        let RegisteredAgent {
            pool,
            mut agent_message_rx,
        } = pool_with_one_free_slot("agent-conversation-history-embeddings");
        let session_controller = open_session_controller().await;

        let continuation_decision = InferenceSocketController::handle_deserialized_message(
            CancellationToken::new(),
            context_with_pool(pool, embeddings_enabled_holder()),
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: "request-conversation-history-embeddings".to_owned(),
                request: InferenceJsonRpcRequest::ContinueFromConversationHistory(
                    ContinueFromConversationHistoryParams {
                        add_generation_prompt: true,
                        conversation_history: ConversationHistory::new(Vec::new()),
                        enable_thinking: false,
                        grammar: None,
                        max_tokens: 1,
                        parse_tool_calls: false,
                        tools: Vec::new(),
                    },
                ),
            }),
            session_controller,
        )
        .await
        .unwrap();

        assert_eq!(
            discriminant(&continuation_decision),
            discriminant(&ContinuationDecision::Continue)
        );
        assert!(agent_message_rx.try_recv().is_err());
    }

    #[actix_web::test]
    async fn handle_error_message_continues_without_dispatch() {
        let RegisteredAgent {
            pool,
            mut agent_message_rx,
        } = pool_with_one_free_slot("agent-error-arm");
        let session_controller = open_session_controller().await;

        let continuation_decision = InferenceSocketController::handle_deserialized_message(
            CancellationToken::new(),
            context_with_pool(pool, Arc::new(BalancerApplicableStateHolder::default())),
            InferenceJsonRpcMessage::Error(ErrorEnvelope {
                request_id: "request-error".to_owned(),
                error: JsonRpcError {
                    code: -32_600,
                    description: "client reported error".to_owned(),
                },
            }),
            session_controller,
        )
        .await
        .unwrap();

        assert_eq!(
            discriminant(&continuation_decision),
            discriminant(&ContinuationDecision::Continue)
        );
        assert!(agent_message_rx.try_recv().is_err());
    }

    #[actix_web::test]
    async fn handle_raw_prompt_request_dispatches_to_agent() {
        let RegisteredAgent {
            pool,
            mut agent_message_rx,
        } = pool_with_one_free_slot("agent-raw-prompt");
        let session_controller = open_session_controller().await;
        let connection_close = CancellationToken::new();

        let continuation_decision = InferenceSocketController::handle_deserialized_message(
            connection_close.clone(),
            context_with_pool(pool, Arc::new(BalancerApplicableStateHolder::default())),
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: "request-raw-prompt".to_owned(),
                request: InferenceJsonRpcRequest::ContinueFromRawPrompt(
                    ContinueFromRawPromptParams {
                        grammar: None,
                        max_tokens: 1,
                        raw_prompt: "fixture prompt".to_owned(),
                    },
                ),
            }),
            session_controller,
        )
        .await
        .unwrap();

        assert_eq!(
            discriminant(&continuation_decision),
            discriminant(&ContinuationDecision::Continue)
        );

        let dispatched_message = agent_message_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&dispatched_message),
            discriminant(&AgentJsonRpcMessage::Request(RequestEnvelope {
                id: "request-raw-prompt".to_owned(),
                request: AgentJsonRpcRequest::ContinueFromRawPrompt(ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: 1,
                    raw_prompt: "fixture prompt".to_owned(),
                }),
            })),
        );

        connection_close.cancel();

        let stop_message = agent_message_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&stop_message),
            discriminant(&AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::StopRespondingTo("request-raw-prompt".to_owned())
            )),
        );
    }

    #[actix_web::test]
    async fn handle_conversation_history_request_validates_and_dispatches_to_agent() {
        let RegisteredAgent {
            pool,
            mut agent_message_rx,
        } = pool_with_one_free_slot("agent-conversation-history");
        let session_controller = open_session_controller().await;
        let connection_close = CancellationToken::new();

        let continuation_decision = InferenceSocketController::handle_deserialized_message(
            connection_close.clone(),
            context_with_pool(pool, Arc::new(BalancerApplicableStateHolder::default())),
            InferenceJsonRpcMessage::Request(RequestEnvelope {
                id: "request-conversation-history".to_owned(),
                request: InferenceJsonRpcRequest::ContinueFromConversationHistory(
                    ContinueFromConversationHistoryParams {
                        add_generation_prompt: true,
                        conversation_history: ConversationHistory::new(Vec::new()),
                        enable_thinking: false,
                        grammar: None,
                        max_tokens: 1,
                        parse_tool_calls: false,
                        tools: Vec::new(),
                    },
                ),
            }),
            session_controller,
        )
        .await
        .unwrap();

        assert_eq!(
            discriminant(&continuation_decision),
            discriminant(&ContinuationDecision::Continue)
        );

        let dispatched_message = agent_message_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&dispatched_message),
            discriminant(&AgentJsonRpcMessage::Request(RequestEnvelope {
                id: "request-conversation-history".to_owned(),
                request: AgentJsonRpcRequest::ContinueFromConversationHistory(
                    ContinueFromConversationHistoryParams {
                        add_generation_prompt: true,
                        conversation_history: ConversationHistory::new(Vec::new()),
                        enable_thinking: false,
                        grammar: None,
                        max_tokens: 1,
                        parse_tool_calls: false,
                        tools: Vec::new(),
                    },
                ),
            })),
        );

        connection_close.cancel();

        let stop_message = agent_message_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&stop_message),
            discriminant(&AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::StopRespondingTo(
                    "request-conversation-history".to_owned()
                )
            )),
        );
    }
}
