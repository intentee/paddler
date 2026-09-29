use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::SinkExt as _;
use futures_util::StreamExt;
use log::debug;
use log::error;
use log::info;
use log::warn;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::continuous_batch_preparation_request::ContinuousBatchPreparationRequest;
use crate::forward_management_socket_messages::forward_management_socket_messages;
use crate::from_request_params::FromRequestParams;
use crate::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::model_metadata_holder::ModelMetadataHolder;
use crate::receive_stream_stopper_collection::ReceiveStreamStopperCollection;
use crate::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::management_socket::agent::message::Message as JsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as JsonRpcNotification;
use paddler_messaging::management_socket::agent::request::Request as JsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as JsonRpcResponse;
use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

struct IncomingMessageContext {
    agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    agent_desired_state_tx: mpsc::UnboundedSender<AgentDesiredState>,
    connection_close: CancellationToken,
    continuous_batch_preparation_request_tx:
        mpsc::UnboundedSender<ContinuousBatchPreparationRequest>,
    model_metadata_holder: Arc<ModelMetadataHolder>,
    receive_stream_stopper_collection: Arc<ReceiveStreamStopperCollection>,
    message_tx: mpsc::UnboundedSender<ManagementJsonRpcMessage>,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

pub struct ManagementSocketClientService {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_desired_state_tx: mpsc::UnboundedSender<AgentDesiredState>,
    pub continuous_batch_preparation_request_tx:
        mpsc::UnboundedSender<ContinuousBatchPreparationRequest>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub name: Option<String>,
    pub receive_stream_stopper_collection: Arc<ReceiveStreamStopperCollection>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    pub socket_url: String,
}

impl ManagementSocketClientService {
    fn generate_responses<TRequest>(
        connection_close: CancellationToken,
        id: String,
        message_tx: mpsc::UnboundedSender<ManagementJsonRpcMessage>,
        request_params: TRequest::RequestParams,
        receive_stream_stopper_collection: Arc<ReceiveStreamStopperCollection>,
        continuous_batch_preparation_request_tx: mpsc::UnboundedSender<
            ContinuousBatchPreparationRequest,
        >,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<()>
    where
        TRequest: FromRequestParams + Into<ContinuousBatchPreparationRequest> + 'static,
        TRequest::Response: Send,
    {
        let (response_tx, mut response_rx) = mpsc::unbounded_channel::<TRequest::Response>();
        let (stop_tx, stop_rx) = mpsc::unbounded_channel::<()>();

        let stopper_guard = receive_stream_stopper_collection
            .register_stopper_with_guard(id.clone(), stop_tx)
            .context(format!("Failed to register stopper for request: {id}"))?;

        continuous_batch_preparation_request_tx.send(
            TRequest::from_request_params(
                request_params,
                response_tx,
                stop_rx,
                slot_aggregated_status,
            )
            .into(),
        )?;

        tokio::spawn(async move {
            let _stopper_guard = stopper_guard;

            loop {
                tokio::select! {
                    () = connection_close.cancelled() => break,
                    response = response_rx.recv() => {
                        match response {
                            Some(response) => {
                                if let Err(err) = message_tx.send(
                                    ManagementJsonRpcMessage::Response(
                                        ResponseEnvelope {
                                            generated_by: None,
                                            request_id: id.clone(),
                                            response: response.into(),
                                        }
                                    ),
                                ) {
                                    error!("Failed to forward response for request {id:?}: {err}");

                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        Ok(())
    }

    fn handle_deserialized_message(
        IncomingMessageContext {
            agent_applicable_state_holder,
            agent_desired_state_tx,
            connection_close,
            continuous_batch_preparation_request_tx,
            message_tx,
            model_metadata_holder,
            receive_stream_stopper_collection,
            slot_aggregated_status,
        }: IncomingMessageContext,
        deserialized_message: JsonRpcMessage,
    ) -> Result<()> {
        match deserialized_message {
            JsonRpcMessage::Notification(JsonRpcNotification::SetState(set_state_params)) => {
                agent_desired_state_tx.send(set_state_params.desired_state)?;

                Ok(())
            }
            JsonRpcMessage::Notification(JsonRpcNotification::StopRespondingTo(request_id)) => {
                debug!("Received StopGeneratingTokens notification for request ID: {request_id:?}");
                receive_stream_stopper_collection
                    .stop(&request_id)
                    .context(format!(
                        "Failed to stop generating tokens for request ID: {request_id}"
                    ))?;

                Ok(())
            }
            JsonRpcMessage::Notification(JsonRpcNotification::Version(VersionParams {
                version,
            })) => {
                if version != env!("CARGO_PKG_VERSION") {
                    warn!(
                        "Version mismatch: server version is {version}, client version is {}",
                        env!("CARGO_PKG_VERSION")
                    );
                }

                Ok(())
            }
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request:
                    JsonRpcRequest::ContinueFromConversationHistory(
                        continue_from_conversation_history_params,
                    ),
            }) => Self::generate_responses::<ContinueFromConversationHistoryRequest>(
                connection_close,
                id,
                message_tx,
                continue_from_conversation_history_params,
                receive_stream_stopper_collection,
                continuous_batch_preparation_request_tx,
                slot_aggregated_status,
            ),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::ContinueFromRawPrompt(generate_tokens_params),
            }) => Self::generate_responses::<ContinueFromRawPromptRequest>(
                connection_close,
                id,
                message_tx,
                generate_tokens_params,
                receive_stream_stopper_collection,
                continuous_batch_preparation_request_tx,
                slot_aggregated_status,
            ),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GenerateEmbeddingBatch(generate_embedding_batch_params),
            }) => Self::generate_responses::<GenerateEmbeddingBatchRequest>(
                connection_close,
                id,
                message_tx,
                generate_embedding_batch_params,
                receive_stream_stopper_collection,
                continuous_batch_preparation_request_tx,
                slot_aggregated_status,
            ),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GetChatTemplateOverride,
            }) => Ok(
                message_tx.send(ManagementJsonRpcMessage::Response(ResponseEnvelope {
                    generated_by: None,
                    request_id: id,
                    response: JsonRpcResponse::ChatTemplateOverride(
                        if let Some(agent_applicable_state) =
                            agent_applicable_state_holder.get_agent_applicable_state()
                        {
                            agent_applicable_state.chat_template_override
                        } else {
                            None
                        },
                    ),
                }))?,
            ),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GetModelMetadata,
            }) => Ok(
                message_tx.send(ManagementJsonRpcMessage::Response(ResponseEnvelope {
                    generated_by: None,
                    request_id: id,
                    response: JsonRpcResponse::ModelMetadata(
                        model_metadata_holder.get_model_metadata(),
                    ),
                }))?,
            ),
        }
    }

    fn handle_incoming_message(incoming_message_context: IncomingMessageContext, msg: Message) {
        match msg {
            Message::Text(text) => match serde_json::from_str::<JsonRpcMessage>(&text) {
                Ok(deserialized_message) => {
                    if let Err(err) = Self::handle_deserialized_message(
                        incoming_message_context,
                        deserialized_message,
                    ) {
                        error!("Error handling incoming message: {err}");
                    }
                }
                Err(err) => {
                    error!("Failed to deserialize JSON-RPC message {text}: {err}");
                }
            },
            Message::Binary(_) | Message::Frame(_) => {
                error!("Received a non-text message, which is not expected");
            }
            Message::Close(_) => {
                info!("Connection closed by server");
            }
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }

    async fn keep_connection_alive(&self, shutdown: CancellationToken) -> Result<()> {
        info!("Connecting to management server at {}", self.socket_url);

        let (ws_stream, _response) = match shutdown
            .run_until_cancelled(connect_async_with_config(
                self.socket_url.clone(),
                None,
                true,
            ))
            .await
        {
            Some(connect_outcome) => connect_outcome?,
            None => return Ok(()),
        };

        info!("Connected to management server");

        let connection_close = CancellationToken::new();
        let (message_tx, message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (write, mut read) = ws_stream.split();

        let message_forward_handle = tokio::spawn(forward_management_socket_messages(
            connection_close.clone(),
            message_rx,
            write,
        ));

        message_tx
            .send(ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                    name: self.name.clone(),
                    slot_aggregated_status_snapshot: self.slot_aggregated_status.make_snapshot(),
                }),
            ))
            .unwrap_or_else(|err| {
                error!("Failed to send register agent notification: {err}");
            });

        let do_send_status_update = || {
            message_tx
                .send(ManagementJsonRpcMessage::Notification(
                    ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                        slot_aggregated_status_snapshot: self
                            .slot_aggregated_status
                            .make_snapshot(),
                    }),
                ))
                .unwrap_or_else(|err| {
                    error!("Failed to send status update notification: {err}");
                });
        };

        let mut ticker = interval(Duration::from_secs(1));
        let mut update_rx = self.slot_aggregated_status.subscribe_to_updates();

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let agent_ends_connection = loop {
            tokio::select! {
                () = connection_close.cancelled() => {
                    info!("Connection close signal received, shutting down");

                    break false;
                }
                () = shutdown.cancelled() => break true,
                changed = update_rx.changed() => {
                    if changed.is_err() {
                        break true;
                    }
                    do_send_status_update();
                }
                _ = ticker.tick() => do_send_status_update(),
                msg = read.next() => {
                    let should_close = match msg {
                        Some(Ok(msg)) => {
                            Self::handle_incoming_message(
                                    IncomingMessageContext {
                                        agent_applicable_state_holder: self.agent_applicable_state_holder.clone(),
                                        agent_desired_state_tx: self.agent_desired_state_tx.clone(),
                                        connection_close: connection_close.clone(),
                                        continuous_batch_preparation_request_tx: self.continuous_batch_preparation_request_tx.clone(),
                                        model_metadata_holder: self.model_metadata_holder.clone(),
                                        receive_stream_stopper_collection: self.receive_stream_stopper_collection.clone(),
                                        message_tx: message_tx.clone(),
                                        slot_aggregated_status: self.slot_aggregated_status.clone(),
                                    },
                                    msg,
                                );

                            false
                        }
                        Some(Err(err)) => {
                            error!("Error reading message: {err}");

                            true
                        }
                        None => true,
                    };

                    if should_close {
                        connection_close.cancel();

                        break false;
                    }
                }
            }
        };

        message_tx
            .send(ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::DeregisterAgent,
            ))
            .unwrap_or_else(|err| {
                error!("Failed to send deregister agent notification: {err}");
            });

        connection_close.cancel();

        let mut write = message_forward_handle
            .await
            .context("Failed to join message forwarding task")?;

        if agent_ends_connection {
            write
                .send(Message::Close(None))
                .await
                .context("Failed to close the management socket")?;
        }

        Ok(())
    }
}

#[async_trait]
impl Service for ManagementSocketClientService {
    fn name(&self) -> &'static str {
        "agent::management_socket_client_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    match self.keep_connection_alive(shutdown.clone()).await {
                        Err(err) => {
                            error!("Failed to keep the connection alive: {err:?}");
                        }
                        Ok(()) => {
                            info!("Management server connection closed");
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use std::collections::BTreeMap;
    use std::sync::Arc;

    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_messaging::management_socket::agent::message::Message as JsonRpcMessage;
    use paddler_messaging::management_socket::agent::notification::Notification as JsonRpcNotification;
    use paddler_messaging::management_socket::agent::notification_params::set_state_params::SetStateParams;
    use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
    use paddler_messaging::management_socket::agent::request::Request as JsonRpcRequest;
    use paddler_messaging::management_socket::agent::response::Response as JsonRpcResponse;
    use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
    use paddler_messaging::model_metadata::ModelMetadata;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio::time::Duration;
    use tokio_tungstenite::tungstenite::protocol::Message;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service;

    use super::IncomingMessageContext;
    use super::ManagementSocketClientService;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
    use crate::continuous_batch_preparation_request::ContinuousBatchPreparationRequest;
    use crate::model_metadata_holder::ModelMetadataHolder;
    use crate::receive_stream_stop_outcome::ReceiveStreamStopOutcome;
    use crate::receive_stream_stopper_collection::ReceiveStreamStopperCollection;
    use crate::slot_aggregated_status::SlotAggregatedStatus;

    const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);

    fn service_with_socket_url(socket_url: String) -> ManagementSocketClientService {
        let (agent_desired_state_tx, _agent_desired_state_rx) = mpsc::unbounded_channel();
        let (continuous_batch_preparation_request_tx, _continuous_batch_preparation_request_rx) =
            mpsc::unbounded_channel();

        ManagementSocketClientService {
            agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            continuous_batch_preparation_request_tx,
            model_metadata_holder: Arc::new(ModelMetadataHolder::new()),
            name: None,
            receive_stream_stopper_collection: Arc::new(ReceiveStreamStopperCollection::default()),
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(2)),
            socket_url,
        }
    }

    struct StalledHandshakeFixture {
        accepted_rx: oneshot::Receiver<()>,
        server: tokio::task::JoinHandle<()>,
        shutdown: CancellationToken,
    }

    fn spawn_stalled_handshake_fixture(listener: TcpListener) -> StalledHandshakeFixture {
        let (accepted_tx, accepted_rx) = oneshot::channel::<()>();
        let shutdown = CancellationToken::new();
        let server_shutdown = shutdown.clone();

        let server = tokio::spawn(async move {
            let (_stream, _peer_addr) = listener
                .accept()
                .await
                .expect("the fixture balancer must accept the agent connection");

            accepted_tx
                .send(())
                .expect("the test must still be waiting for the accept signal");

            server_shutdown.cancelled().await;
        });

        StalledHandshakeFixture {
            accepted_rx,
            server,
            shutdown,
        }
    }

    #[tokio::test]
    async fn keep_connection_alive_returns_when_shutdown_arrives_during_a_stalled_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let StalledHandshakeFixture {
            accepted_rx,
            server,
            shutdown: fixture_shutdown,
        } = spawn_stalled_handshake_fixture(listener);

        let service =
            service_with_socket_url(format!("ws://{addr}/api/v1/agent_socket/test-agent"));
        let shutdown = CancellationToken::new();
        let keep_alive_shutdown = shutdown.clone();
        let keep_alive_handle =
            tokio::spawn(async move { service.keep_connection_alive(keep_alive_shutdown).await });

        accepted_rx
            .await
            .expect("the agent's connect must reach the fixture balancer before shutdown");

        shutdown.cancel();

        let keep_alive_result = tokio::time::timeout(SHUTDOWN_BUDGET, keep_alive_handle)
            .await
            .expect(
                "keep_connection_alive must abandon a stalled connect handshake when shutdown \
                 arrives instead of blocking until trzcina force-aborts it",
            )
            .expect("the keep_connection_alive task must not panic");

        assert!(keep_alive_result.is_ok());

        fixture_shutdown.cancel();
        server
            .await
            .expect("the fixture balancer task must not panic");
    }

    #[tokio::test]
    async fn keep_connection_alive_errors_when_the_balancer_refuses_the_connection() {
        let service =
            service_with_socket_url("ws://127.0.0.1:1/api/v1/agent_socket/test-agent".to_owned());
        let shutdown = CancellationToken::new();

        let keep_alive_result =
            tokio::time::timeout(SHUTDOWN_BUDGET, service.keep_connection_alive(shutdown))
                .await
                .expect("connecting to a refused port must fail fast instead of blocking");

        assert!(keep_alive_result.is_err());
    }

    #[tokio::test]
    async fn run_returns_when_shutdown_arrives_during_a_stalled_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let StalledHandshakeFixture {
            accepted_rx,
            server,
            shutdown: fixture_shutdown,
        } = spawn_stalled_handshake_fixture(listener);

        let service =
            service_with_socket_url(format!("ws://{addr}/api/v1/agent_socket/test-agent"));
        let shutdown = CancellationToken::new();
        let run_shutdown = shutdown.clone();
        let run_handle = tokio::spawn(async move { Box::new(service).run(run_shutdown).await });

        accepted_rx
            .await
            .expect("the agent's connect must reach the fixture balancer before shutdown");

        shutdown.cancel();

        let run_result = tokio::time::timeout(SHUTDOWN_BUDGET, run_handle)
            .await
            .expect(
                "run must return promptly on shutdown even while a connect handshake is stalled, \
                 instead of blocking until trzcina force-aborts the service",
            )
            .expect("the run task must not panic");

        assert!(run_result.is_ok());

        fixture_shutdown.cancel();
        server
            .await
            .expect("the fixture balancer task must not panic");
    }

    fn build_incoming_message_context(
        agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
        agent_desired_state_tx: mpsc::UnboundedSender<AgentDesiredState>,
        connection_close: CancellationToken,
        model_metadata_holder: Arc<ModelMetadataHolder>,
        receive_stream_stopper_collection: Arc<ReceiveStreamStopperCollection>,
        message_tx: mpsc::UnboundedSender<ManagementJsonRpcMessage>,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> IncomingMessageContext {
        let (continuous_batch_preparation_request_tx, _continuous_batch_preparation_request_rx) =
            mpsc::unbounded_channel::<ContinuousBatchPreparationRequest>();

        IncomingMessageContext {
            agent_applicable_state_holder,
            agent_desired_state_tx,
            connection_close,
            continuous_batch_preparation_request_tx,
            model_metadata_holder,
            receive_stream_stopper_collection,
            message_tx,
            slot_aggregated_status,
        }
    }

    #[tokio::test]
    async fn set_state_notification_forwards_desired_state() {
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, mut agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            Arc::new(ReceiveStreamStopperCollection::default()),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let result = ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Notification(JsonRpcNotification::SetState(Box::new(SetStateParams {
                desired_state: AgentDesiredState::default(),
            }))),
        );

        assert!(result.is_ok());
        assert_eq!(
            agent_desired_state_rx.try_recv().unwrap(),
            AgentDesiredState::default()
        );
    }

    #[tokio::test]
    async fn set_state_notification_errors_when_receiver_dropped() {
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();

        drop(agent_desired_state_rx);

        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            Arc::new(ReceiveStreamStopperCollection::default()),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let result = ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Notification(JsonRpcNotification::SetState(Box::new(SetStateParams {
                desired_state: AgentDesiredState::default(),
            }))),
        );

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stop_responding_to_a_finished_request_is_not_an_error_and_retains_nothing() {
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, _agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let receive_stream_stopper_collection = Arc::new(ReceiveStreamStopperCollection::default());
        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            receive_stream_stopper_collection.clone(),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Notification(JsonRpcNotification::StopRespondingTo(
                "already_finished".to_owned(),
            )),
        )
        .expect("a stop for a request that already finished must not be an error");

        assert!(
            receive_stream_stopper_collection
                .deregister_stopper("already_finished")
                .is_err(),
            "the stop must leave nothing behind; retaining it would leak one entry per cancelled \
             request"
        );
    }

    #[tokio::test]
    async fn stop_responding_to_registered_request_signals_stopper() {
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, _agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let receive_stream_stopper_collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<()>();

        receive_stream_stopper_collection
            .register_stopper("active_request".to_owned(), stop_tx)
            .unwrap();

        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            receive_stream_stopper_collection,
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let result = ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Notification(JsonRpcNotification::StopRespondingTo(
                "active_request".to_owned(),
            )),
        );

        assert!(result.is_ok());
        assert!(stop_rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn mismatched_version_notification_is_acknowledged() {
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, _agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            Arc::new(ReceiveStreamStopperCollection::default()),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let result = ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Notification(JsonRpcNotification::Version(VersionParams {
                version: "0.0.0-mismatch".to_owned(),
            })),
        );

        assert!(result.is_ok());
        assert!(message_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn get_chat_template_override_without_applicable_state_responds_with_none() {
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, _agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            Arc::new(ReceiveStreamStopperCollection::default()),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let result = ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Request(RequestEnvelope {
                id: "req_template".to_owned(),
                request: JsonRpcRequest::GetChatTemplateOverride,
            }),
        );

        assert!(result.is_ok());

        let sent_message = message_rx.try_recv().unwrap();

        assert!(matches!(
            sent_message,
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: JsonRpcResponse::ChatTemplateOverride(None),
                ..
            }) if request_id == "req_template"
        ));
    }

    #[tokio::test]
    async fn get_chat_template_override_errors_when_message_receiver_dropped() {
        let (message_tx, message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, _agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();

        drop(message_rx);

        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            Arc::new(ReceiveStreamStopperCollection::default()),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let result = ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Request(RequestEnvelope {
                id: "req_template".to_owned(),
                request: JsonRpcRequest::GetChatTemplateOverride,
            }),
        );

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_model_metadata_responds_with_stored_metadata() {
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, _agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let model_metadata_holder = Arc::new(ModelMetadataHolder::new());
        let mut metadata = BTreeMap::new();

        metadata.insert("architecture".to_owned(), "llama".to_owned());
        model_metadata_holder.set_model_metadata(ModelMetadata {
            metadata: metadata.clone(),
        });

        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            model_metadata_holder,
            Arc::new(ReceiveStreamStopperCollection::default()),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let result = ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Request(RequestEnvelope {
                id: "req_metadata".to_owned(),
                request: JsonRpcRequest::GetModelMetadata,
            }),
        );

        assert!(result.is_ok());

        let sent_message = message_rx.try_recv().unwrap();

        assert!(matches!(
            sent_message,
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                response: JsonRpcResponse::ModelMetadata(Some(returned_metadata)),
                ..
            }) if returned_metadata.metadata == metadata
        ));
    }

    #[tokio::test]
    async fn get_model_metadata_errors_when_message_receiver_dropped() {
        let (message_tx, message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, _agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();

        drop(message_rx);

        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            Arc::new(ReceiveStreamStopperCollection::default()),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let result = ManagementSocketClientService::handle_deserialized_message(
            context,
            JsonRpcMessage::Request(RequestEnvelope {
                id: "req_metadata".to_owned(),
                request: JsonRpcRequest::GetModelMetadata,
            }),
        );

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn text_message_dispatches_deserialized_set_state() {
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (agent_desired_state_tx, mut agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let context = build_incoming_message_context(
            Arc::new(AgentApplicableStateHolder::default()),
            agent_desired_state_tx,
            CancellationToken::new(),
            Arc::new(ModelMetadataHolder::new()),
            Arc::new(ReceiveStreamStopperCollection::default()),
            message_tx,
            Arc::new(SlotAggregatedStatus::new(2)),
        );

        let serialized_set_state = serde_json::to_string(&JsonRpcMessage::Notification(
            JsonRpcNotification::SetState(Box::new(SetStateParams {
                desired_state: AgentDesiredState::default(),
            })),
        ))
        .unwrap();

        ManagementSocketClientService::handle_incoming_message(
            context,
            Message::Text(serialized_set_state.into()),
        );

        assert_eq!(
            agent_desired_state_rx.recv().await.unwrap(),
            AgentDesiredState::default()
        );
    }

    #[tokio::test]
    async fn generate_responses_registers_the_stopper_before_it_returns() {
        let connection_close = CancellationToken::new();
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let receive_stream_stopper_collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (request_tx, mut request_rx) =
            mpsc::unbounded_channel::<ContinuousBatchPreparationRequest>();
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(2));

        ManagementSocketClientService::generate_responses::<ContinueFromRawPromptRequest>(
            connection_close,
            "req_generate".to_owned(),
            message_tx,
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(8).unwrap(),
                raw_prompt: "hello".to_owned(),
            },
            receive_stream_stopper_collection.clone(),
            request_tx,
            slot_aggregated_status,
        )
        .expect("the request must be accepted");

        let dispatched_request = request_rx
            .try_recv()
            .expect("the request must be dispatched to the arbiter");

        assert!(matches!(
            &dispatched_request,
            ContinuousBatchPreparationRequest::ContinueFromRawPrompt(request)
                if request.params.raw_prompt == "hello"
        ));
        assert_eq!(
            receive_stream_stopper_collection
                .stop("req_generate")
                .unwrap(),
            ReceiveStreamStopOutcome::StopSignalled,
            "the stopper must be registered before generate_responses returns, so a stop arriving \
             on the next frame cannot be lost"
        );
    }

    #[tokio::test]
    async fn generate_responses_errors_when_request_receiver_dropped() {
        let connection_close = CancellationToken::new();
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let receive_stream_stopper_collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (request_tx, request_rx) =
            mpsc::unbounded_channel::<ContinuousBatchPreparationRequest>();
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(2));

        drop(request_rx);

        let result =
            ManagementSocketClientService::generate_responses::<ContinueFromRawPromptRequest>(
                connection_close,
                "req_generate".to_owned(),
                message_tx,
                ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::new(8).unwrap(),
                    raw_prompt: "hello".to_owned(),
                },
                receive_stream_stopper_collection,
                request_tx,
                slot_aggregated_status,
            );

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn generate_responses_errors_when_stopper_already_registered() {
        let connection_close = CancellationToken::new();
        let (message_tx, _message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let receive_stream_stopper_collection = Arc::new(ReceiveStreamStopperCollection::default());
        let (existing_stop_tx, _existing_stop_rx) = mpsc::unbounded_channel::<()>();
        let (request_tx, _request_rx) =
            mpsc::unbounded_channel::<ContinuousBatchPreparationRequest>();
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(2));

        receive_stream_stopper_collection
            .register_stopper("req_generate".to_owned(), existing_stop_tx)
            .unwrap();

        let result =
            ManagementSocketClientService::generate_responses::<ContinueFromRawPromptRequest>(
                connection_close,
                "req_generate".to_owned(),
                message_tx,
                ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::new(8).unwrap(),
                    raw_prompt: "hello".to_owned(),
                },
                receive_stream_stopper_collection,
                request_tx,
                slot_aggregated_status,
            );

        assert!(result.is_err());
    }
}
