use std::fmt::Debug;
use std::sync::Arc;

use crate::balancer::inference_client::Message as OutgoingMessage;
use crate::balancer::inference_client::Response as OutgoingResponse;
use crate::jsonrpc::Error as JsonRpcError;
use crate::jsonrpc::ErrorEnvelope;
use crate::jsonrpc::ResponseEnvelope;
use crate::streamable_result::StreamableResult;
use log::debug;
use log::error;
use log::warn;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::agent::jsonrpc::Request as AgentJsonRpcRequest;
use crate::balancer::agent_controller::AgentController;
use crate::balancer::buffered_request_agent_wait_result::BufferedRequestAgentWaitResult;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::dispatched_agent::DispatchedAgent;
use crate::balancer::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::balancer::manages_senders::ManagesSenders;
use crate::balancer::manages_senders_controller::ManagesSendersController;
use crate::controls_session::ControlsSession;

pub async fn request_from_agent<TControlsSession, TParams>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    connection_close: CancellationToken,
    inference_service_configuration: InferenceServiceConfiguration,
    params: TParams,
    request_id: String,
    mut session_controller: TControlsSession,
    shutdown: CancellationToken,
)
where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TParams: Debug + Into<AgentJsonRpcRequest> + Send,
    AgentController: HandlesAgentStreamingResponse<TParams>,
    <<AgentController as HandlesAgentStreamingResponse<TParams>>::SenderCollection as ManagesSenders>::Value: Debug + Into<OutgoingResponse> + StreamableResult,
{
    let Some(dispatched_agent) = wait_for_agent_controller(
        buffered_request_manager.clone(),
        connection_close.clone(),
        request_id.clone(),
        &mut session_controller,
        shutdown.clone(),
    )
    .await
    else {
        return;
    };

    let receive_response_controller = match dispatched_agent
        .agent_controller
        .handle_streaming_response(request_id.clone(), params)
        .await
    {
        Ok(receive_response_controller) => receive_response_controller,
        Err(err) => {
            error!("Failed to handle request {request_id:?}: {err}");

            respond_with_error(
                JsonRpcError {
                    code: 500,
                    description: "Failed to generate response".to_owned(),
                },
                request_id.clone(),
                &mut session_controller,
            )
            .await;

            return;
        }
    };

    forward_responses_stream(
        dispatched_agent.agent_controller.clone(),
        connection_close,
        inference_service_configuration,
        receive_response_controller,
        request_id,
        session_controller,
        shutdown,
    )
    .await;
}

pub async fn forward_responses_stream<TControlsSession, TManagesSenders>(
    agent_controller: Arc<AgentController>,
    connection_close: CancellationToken,
    inference_service_configuration: InferenceServiceConfiguration,
    mut receive_response_controller: ManagesSendersController<TManagesSenders>,
    request_id: String,
    mut session_controller: TControlsSession,
    shutdown: CancellationToken,
) where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TManagesSenders: ManagesSenders + Send + Sync,
    TManagesSenders::Value: Debug + Into<OutgoingResponse> + Send + StreamableResult,
{
    debug!("Found available agent controller for request: {request_id:?}");

    let agent_connection_close = agent_controller.connection_close.clone();

    loop {
        tokio::select! {
            () = agent_connection_close.cancelled() => {
                error!("Agent controller connection closed");

                respond_with_error(
                    JsonRpcError {
                        code: 502,
                        description: "Agent controller connection closed".to_owned(),
                    },
                    request_id,
                    &mut session_controller,
                ).await;

                break;
            }
            () = connection_close.cancelled() => {
                agent_controller.stop_responding_to(request_id.clone()).await.unwrap_or_else(|err| {
                    error!("Failed to stop request {request_id:?}: {err}");
                });

                break;
            }
            () = shutdown.cancelled() => {
                respond_with_error(
                    JsonRpcError {
                        code: 503,
                        description: "balancer is shutting down".to_owned(),
                    },
                    request_id.clone(),
                    &mut session_controller,
                ).await;

                agent_controller.stop_responding_to(request_id.clone()).await.unwrap_or_else(|err| {
                    error!("Failed to stop request {request_id:?}: {err}");
                });

                break;
            }
            () = sleep(inference_service_configuration.inference_item_timeout) => {
                let timeout_ms = inference_service_configuration.inference_item_timeout.as_millis();

                warn!(
                    "Timed out after {timeout_ms}ms waiting for next token for request {request_id:?}. \
                    Consider increasing --inference-item-timeout if the model needs more time to process the prompt."
                );

                respond_with_error(
                    JsonRpcError {
                        code: 504,
                        description: format!(
                            "Inference timed out after {timeout_ms}ms waiting for next token. \
                            Increase --inference-item-timeout if the prompt requires longer processing."
                        ),
                    },
                    request_id.clone(),
                    &mut session_controller,
                ).await;

                agent_controller.stop_responding_to(request_id.clone()).await.unwrap_or_else(|err| {
                    error!("Failed to stop responding to request {request_id:?}: {err}");
                });

                break;
            }
            response = receive_response_controller.response_rx.recv() => {
                if let Some(response) = response {
                    let is_done = response.is_done();

                    let send_succeeded = send_response_to_client(
                        agent_controller.clone(),
                        response,
                        request_id.clone(),
                        &mut session_controller,
                    ).await;

                    if is_done || !send_succeeded {
                        break;
                    }
                } else {
                    error!(
                        "Response channel closed before terminator for request {request_id:?}"
                    );

                    respond_with_error(
                        JsonRpcError {
                            code: 502,
                            description:
                                "Response channel closed before terminator".to_owned(),
                        },
                        request_id,
                        &mut session_controller,
                    ).await;

                    break;
                }
            }
        }
    }
}

pub async fn respond_with_error<TControlsSession>(
    error: JsonRpcError,
    request_id: String,
    session_controller: &mut TControlsSession,
) where
    TControlsSession: ControlsSession<OutgoingMessage>,
{
    session_controller
        .send_response(OutgoingMessage::Error(ErrorEnvelope {
            request_id: request_id.clone(),
            error,
        }))
        .await
        .unwrap_or_else(|err| {
            error!("Failed to send response for request {request_id:?}: {err}");
        });
}

async fn send_response_to_client<TControlsSession, TResponse>(
    agent_controller: Arc<AgentController>,
    response: TResponse,
    request_id: String,
    session_controller: &mut TControlsSession,
) -> bool
where
    TControlsSession: ControlsSession<OutgoingMessage>,
    TResponse: Into<OutgoingResponse> + Send,
{
    if let Err(err) = session_controller
        .send_response(OutgoingMessage::Response(ResponseEnvelope {
            generated_by: agent_controller.name.clone(),
            request_id: request_id.clone(),
            response: response.into(),
        }))
        .await
    {
        error!("Failed to send response for request {request_id:?}: {err}");

        agent_controller
            .stop_responding_to(request_id.clone())
            .await
            .unwrap_or_else(|err| {
                error!("Failed to stop responding to request {request_id:?}: {err}");
            });

        return false;
    }

    true
}

async fn wait_for_agent_controller<TControlsSession>(
    buffered_request_manager: Arc<BufferedRequestManager>,
    connection_close: CancellationToken,
    request_id: String,
    session_controller: &mut TControlsSession,
    shutdown: CancellationToken,
) -> Option<DispatchedAgent>
where
    TControlsSession: ControlsSession<OutgoingMessage>,
{
    let buffered_request_manager = buffered_request_manager.clone();

    tokio::select! {
        () = connection_close.cancelled() => {
            debug!("Connection close signal received, stopping GenerateTokens loop.");

            None
        },
        () = shutdown.cancelled() => {
            respond_with_error(
                JsonRpcError {
                    code: 503,
                    description: "balancer is shutting down".to_owned(),
                },
                request_id.clone(),
                session_controller,
            ).await;

            None
        },
        buffered_request_agent_wait_result = buffered_request_manager.wait_for_available_agent() => {
            match buffered_request_agent_wait_result {
                Ok(BufferedRequestAgentWaitResult::Found(dispatched_agent)) => Some(dispatched_agent),
                Ok(BufferedRequestAgentWaitResult::BufferOverflow) => {
                    warn!("Too many buffered requests, dropping request: {request_id:?}");

                    respond_with_error(
                        JsonRpcError {
                            code: 503,
                            description: "Buffered requests overflow".to_owned(),
                        },
                        request_id.clone(),
                        session_controller,
                    ).await;

                    None
                }
                Ok(BufferedRequestAgentWaitResult::Timeout(err)) => {
                    warn!("Buffered request {request_id:?} timed out: {err:?}");

                    respond_with_error(
                        JsonRpcError {
                            code: 504,
                            description: "Waiting for available slot timed out".to_owned(),
                        },
                        request_id.clone(),
                        session_controller,
                    ).await;

                    None
                }
                Err(err) => {
                    error!("Error while waiting for available agent controller for GenerateTokens request: {err}");

                    respond_with_error(
                        JsonRpcError {
                            code: 500,
                            description: "Internal server error".to_owned(),
                        },
                        request_id.clone(),
                        session_controller,
                    ).await;

                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::mem::discriminant;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    use tokio::sync::mpsc;

    use super::*;
    use crate::agent::jsonrpc::Message as AgentJsonRpcMessage;
    use crate::agent::jsonrpc::Notification as AgentJsonRpcNotification;
    use crate::agent_state_application_status::AgentStateApplicationStatus;
    use crate::atomic_value::AtomicValue;
    use crate::balancer::agent_controller_pool::AgentControllerPool;
    use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::balancer::chunk_forwarding_session_controller::ChunkForwardingSessionController;
    use crate::balancer::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
    use crate::balancer::chunk_forwarding_session_controller::transform_result::TransformResult;
    use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::generated_token_result::GeneratedTokenResult;
    use crate::request_params::ContinueFromRawPromptParams;

    struct AgentControllerWithIncomingChannel {
        agent_controller: Arc<AgentController>,
        agent_message_rx: mpsc::UnboundedReceiver<AgentJsonRpcMessage>,
    }

    fn agent_controller_with_one_free_slot(id: &str) -> AgentControllerWithIncomingChannel {
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
            id: id.to_owned(),
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

        AgentControllerWithIncomingChannel {
            agent_controller,
            agent_message_rx,
        }
    }

    fn raw_prompt_params() -> ContinueFromRawPromptParams {
        ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 1,
            raw_prompt: "fixture prompt".to_owned(),
        }
    }

    fn inference_service_configuration_with_long_timeout() -> InferenceServiceConfiguration {
        const TIMEOUT_LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);

        InferenceServiceConfiguration {
            addr: "127.0.0.1:0".parse().unwrap(),
            cors_allowed_hosts: Vec::new(),
            inference_item_timeout: TIMEOUT_LONGER_THAN_ANY_TEST_RUN,
        }
    }

    #[tokio::test]
    async fn request_from_agent_forwards_error_when_agent_connection_closes() {
        let pool = Arc::new(AgentControllerPool::default());
        let AgentControllerWithIncomingChannel {
            agent_controller,
            agent_message_rx: _agent_message_rx,
        } = agent_controller_with_one_free_slot("agent-close");

        agent_controller.connection_close.cancel();

        pool.register_agent_controller("agent-close".to_owned(), agent_controller)
            .unwrap();

        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        request_from_agent(
            buffered_request_manager,
            CancellationToken::new(),
            inference_service_configuration_with_long_timeout(),
            raw_prompt_params(),
            "request-close".to_owned(),
            session_controller,
            CancellationToken::new(),
        )
        .await;

        let forwarded = chunk_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&forwarded),
            discriminant(&TransformResult::Chunk(String::new()))
        );
    }

    #[tokio::test]
    async fn request_from_agent_responds_with_error_when_streaming_setup_fails() {
        let pool = Arc::new(AgentControllerPool::default());
        let AgentControllerWithIncomingChannel {
            agent_controller,
            agent_message_rx,
        } = agent_controller_with_one_free_slot("agent-setup-fail");

        drop(agent_message_rx);

        pool.register_agent_controller("agent-setup-fail".to_owned(), agent_controller)
            .unwrap();

        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        request_from_agent(
            buffered_request_manager,
            CancellationToken::new(),
            inference_service_configuration_with_long_timeout(),
            raw_prompt_params(),
            "request-setup-fail".to_owned(),
            session_controller,
            CancellationToken::new(),
        )
        .await;

        let forwarded = chunk_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&forwarded),
            discriminant(&TransformResult::Chunk(String::new()))
        );
    }

    #[tokio::test]
    async fn forward_responses_stream_stops_responding_when_client_send_fails() {
        let AgentControllerWithIncomingChannel {
            agent_controller,
            mut agent_message_rx,
        } = agent_controller_with_one_free_slot("agent-send-fail");

        let request_id = "request-send-fail".to_owned();
        let sender_collection = agent_controller.generate_tokens_sender_collection.clone();
        let receive_response_controller = ManagesSendersController::from_request_id(
            request_id.clone(),
            sender_collection.clone(),
        )
        .unwrap();

        sender_collection
            .forward_response(
                request_id.clone(),
                GeneratedTokenResult::ContentToken("token".to_owned()),
            )
            .await
            .unwrap();

        let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

        drop(chunk_rx);

        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        forward_responses_stream(
            agent_controller,
            CancellationToken::new(),
            inference_service_configuration_with_long_timeout(),
            receive_response_controller,
            request_id,
            session_controller,
            CancellationToken::new(),
        )
        .await;

        let stop_message = agent_message_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&stop_message),
            discriminant(&AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::StopRespondingTo(String::new())
            ))
        );
    }

    #[tokio::test]
    async fn request_from_agent_responds_with_error_on_shutdown_while_waiting() {
        let pool = Arc::new(AgentControllerPool::default());
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        let shutdown = CancellationToken::new();

        shutdown.cancel();

        request_from_agent(
            buffered_request_manager,
            CancellationToken::new(),
            inference_service_configuration_with_long_timeout(),
            raw_prompt_params(),
            "request-shutdown".to_owned(),
            session_controller,
            shutdown,
        )
        .await;

        let forwarded = chunk_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&forwarded),
            discriminant(&TransformResult::Chunk(String::new()))
        );
    }

    #[tokio::test]
    async fn request_from_agent_responds_with_error_on_buffer_overflow() {
        let pool = Arc::new(AgentControllerPool::default());
        let AgentControllerWithIncomingChannel {
            agent_controller,
            agent_message_rx: _agent_message_rx,
        } = agent_controller_with_one_free_slot("agent-overflow");

        agent_controller.slots_processing.set(1);

        pool.register_agent_controller("agent-overflow".to_owned(), agent_controller)
            .unwrap();

        let buffered_request_manager =
            Arc::new(BufferedRequestManager::new(pool, Duration::from_secs(1), 0));

        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        request_from_agent(
            buffered_request_manager,
            CancellationToken::new(),
            inference_service_configuration_with_long_timeout(),
            raw_prompt_params(),
            "request-overflow".to_owned(),
            session_controller,
            CancellationToken::new(),
        )
        .await;

        let forwarded = chunk_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&forwarded),
            discriminant(&TransformResult::Chunk(String::new()))
        );
    }

    #[tokio::test]
    async fn request_from_agent_stops_waiting_when_connection_closes() {
        let pool = Arc::new(AgentControllerPool::default());
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            pool,
            Duration::from_secs(1),
            10,
        ));

        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        let connection_close = CancellationToken::new();

        connection_close.cancel();

        request_from_agent(
            buffered_request_manager,
            connection_close,
            inference_service_configuration_with_long_timeout(),
            raw_prompt_params(),
            "request-connection-close".to_owned(),
            session_controller,
            CancellationToken::new(),
        )
        .await;

        assert!(chunk_rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn forward_responses_stream_stops_agent_when_client_connection_closes() {
        let AgentControllerWithIncomingChannel {
            agent_controller,
            agent_message_rx,
        } = agent_controller_with_one_free_slot("agent-client-close");

        drop(agent_message_rx);

        let request_id = "request-client-close".to_owned();
        let receive_response_controller = ManagesSendersController::from_request_id(
            request_id.clone(),
            agent_controller.generate_tokens_sender_collection.clone(),
        )
        .unwrap();

        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        let connection_close = CancellationToken::new();

        connection_close.cancel();

        forward_responses_stream(
            agent_controller,
            connection_close,
            inference_service_configuration_with_long_timeout(),
            receive_response_controller,
            request_id,
            session_controller,
            CancellationToken::new(),
        )
        .await;

        assert!(chunk_rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn forward_responses_stream_responds_with_error_on_shutdown() {
        let AgentControllerWithIncomingChannel {
            agent_controller,
            agent_message_rx,
        } = agent_controller_with_one_free_slot("agent-shutdown");

        drop(agent_message_rx);

        let request_id = "request-stream-shutdown".to_owned();
        let receive_response_controller = ManagesSendersController::from_request_id(
            request_id.clone(),
            agent_controller.generate_tokens_sender_collection.clone(),
        )
        .unwrap();

        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
        let session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        let shutdown = CancellationToken::new();

        shutdown.cancel();

        forward_responses_stream(
            agent_controller,
            CancellationToken::new(),
            inference_service_configuration_with_long_timeout(),
            receive_response_controller,
            request_id,
            session_controller,
            shutdown,
        )
        .await;

        let forwarded = chunk_rx.recv().await.unwrap();

        assert_eq!(
            discriminant(&forwarded),
            discriminant(&TransformResult::Chunk(String::new()))
        );
    }

    #[tokio::test]
    async fn respond_with_error_logs_when_client_send_fails() {
        let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

        drop(chunk_rx);

        let mut session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        respond_with_error(
            JsonRpcError {
                code: 500,
                description: "send fails".to_owned(),
            },
            "request-send-error".to_owned(),
            &mut session_controller,
        )
        .await;
    }

    #[tokio::test]
    async fn send_response_to_client_returns_false_and_logs_when_stop_fails() {
        let AgentControllerWithIncomingChannel {
            agent_controller,
            agent_message_rx,
        } = agent_controller_with_one_free_slot("agent-stop-fails");

        drop(agent_message_rx);

        let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

        drop(chunk_rx);

        let mut session_controller =
            ChunkForwardingSessionController::new(chunk_tx, IdentityTransformer::new());

        let send_succeeded = send_response_to_client(
            agent_controller,
            GeneratedTokenResult::ContentToken("token".to_owned()),
            "request-stop-fails".to_owned(),
            &mut session_controller,
        )
        .await;

        assert!(!send_succeeded);
    }
}
