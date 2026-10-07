use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::SinkExt as _;
use futures_util::StreamExt;
use log::debug;
use log::error;
use log::info;
use log::warn;
use serde_json::from_str;
use tokio::net::TcpStream;
use tokio::select;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::watch;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_messaging::balancer_connection::BalancerConnection;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as JsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as JsonRpcNotification;
use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
use paddler_messaging::management_socket::agent::request::Request as JsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as JsonRpcResponse;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;
use paddler_request_registry::request_delivery::RequestDelivery;
use paddler_request_registry::request_registry_guard::RequestRegistryGuard;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_status::slot_guard::SlotGuard;

use crate::balancer_message_context::BalancerMessageContext;
use crate::forward_management_socket_messages::forward_management_socket_messages;
use crate::pipeline_request::PipelineRequest;

const BALANCER_RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

struct IncomingMessageContext {
    balancer_message_context: BalancerMessageContext,
    connection_close: CancellationToken,
    message_tx: mpsc::UnboundedSender<ManagementJsonRpcMessage>,
}

impl IncomingMessageContext {
    fn generate_responses<TParams, TResponse>(self, id: String, params: TParams) -> Result<()>
    where
        AgentRequest<TParams, TResponse>: Into<PipelineRequest>,
        TResponse: Into<JsonRpcResponse> + Send + 'static,
    {
        let Self {
            balancer_message_context:
                BalancerMessageContext {
                    pipeline_request_tx,
                    request_stoppers,
                    slot_aggregated_status,
                    ..
                },
            connection_close,
            message_tx,
        } = self;
        let (response_tx, mut response_rx) = mpsc::unbounded_channel::<TResponse>();
        let (stop_tx, stop_rx) = mpsc::unbounded_channel::<()>();

        let stopper_guard = RequestRegistryGuard::register(&request_stoppers, id.clone(), stop_tx)
            .context(format!("Failed to register stopper for request: {id}"))?;

        pipeline_request_tx.send(
            AgentRequest {
                params,
                response_tx,
                slot_guard: SlotGuard::new(slot_aggregated_status),
                stop_rx,
            }
            .into(),
        )?;

        spawn(async move {
            let _stopper_guard = stopper_guard;

            loop {
                select! {
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

    fn handle_deserialized_message(self, deserialized_message: JsonRpcMessage) -> Result<()> {
        match deserialized_message {
            JsonRpcMessage::Notification(JsonRpcNotification::SetState(set_state_params)) => {
                self.balancer_message_context
                    .agent_desired_state_tx
                    .send(set_state_params.desired_state)?;

                Ok(())
            }
            JsonRpcMessage::Notification(JsonRpcNotification::StopRespondingTo(request_id)) => {
                debug!("Received StopGeneratingTokens notification for request ID: {request_id:?}");
                match self
                    .balancer_message_context
                    .request_stoppers
                    .send_to(&request_id, ())
                {
                    RequestDelivery::Delivered => Ok(()),
                    RequestDelivery::ReceiverDropped | RequestDelivery::RequestNotRegistered => {
                        debug!("Request {request_id:?} finished before its stop arrived");

                        Ok(())
                    }
                }
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
            }) => self.generate_responses::<_, GeneratedTokenResult>(
                id,
                continue_from_conversation_history_params,
            ),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::ContinueFromRawPrompt(generate_tokens_params),
            }) => self.generate_responses::<_, GeneratedTokenResult>(id, generate_tokens_params),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::Decide(decide_params),
            }) => self.generate_responses::<_, DecisionResult>(id, decide_params),
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GenerateEmbeddingBatch(generate_embedding_batch_params),
            }) => {
                self.generate_responses::<_, EmbeddingResult>(id, generate_embedding_batch_params)
            }
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GetChatTemplateOverride,
            }) => {
                Ok(self
                    .message_tx
                    .send(ManagementJsonRpcMessage::Response(ResponseEnvelope {
                        generated_by: None,
                        request_id: id,
                        response: JsonRpcResponse::ChatTemplateOverride(
                            self.balancer_message_context
                                .agent_applicable_state_holder
                                .get_agent_applicable_state()
                                .chat_template_override(),
                        ),
                    }))?)
            }
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GetModelMetadata,
            }) => {
                Ok(self
                    .message_tx
                    .send(ManagementJsonRpcMessage::Response(ResponseEnvelope {
                        generated_by: None,
                        request_id: id,
                        response: JsonRpcResponse::ModelMetadata(
                            self.balancer_message_context
                                .model_metadata_holder
                                .get_model_metadata(),
                        ),
                    }))?)
            }
        }
    }

    fn handle_incoming_message(self, msg: Message) {
        match msg {
            Message::Text(text) => match from_str::<JsonRpcMessage>(&text) {
                Ok(deserialized_message) => {
                    if let Err(err) = self.handle_deserialized_message(deserialized_message) {
                        error!("Error handling incoming message: {err}");
                    }
                }
                Err(deserialization_error) => {
                    error!(
                        "Closing the management connection after an undeserializable message {text}: {deserialization_error}"
                    );

                    self.connection_close.cancel();
                }
            },
            Message::Binary(_) | Message::Frame(_) => {
                error!("Closing the management connection after a non-text message");

                self.connection_close.cancel();
            }
            Message::Close(_) => {
                info!("Connection closed by server");
            }
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }
}

pub struct ManagementSocketClientService {
    pub balancer_connection_tx: watch::Sender<BalancerConnection>,
    pub balancer_message_context: BalancerMessageContext,
    pub name: Option<String>,
    pub socket_url: String,
}

impl ManagementSocketClientService {
    async fn keep_connection_alive(&self, shutdown: CancellationToken) -> Result<()> {
        match shutdown
            .run_until_cancelled(
                self.balancer_message_context
                    .slot_aggregated_status
                    .wait_until_no_slots_are_processing(),
            )
            .await
        {
            Some(idle_outcome) => idle_outcome
                .context("Slot status stopped announcing updates while waiting for idle slots")?,
            None => return Ok(()),
        }

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

        self.balancer_connection_tx
            .send_replace(BalancerConnection::Connected);

        let connection_outcome = self.serve_connection(ws_stream, shutdown).await;

        self.balancer_connection_tx
            .send_replace(BalancerConnection::Connecting);

        connection_outcome
    }

    async fn serve_connection(
        &self,
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let connection_close = CancellationToken::new();
        let (message_tx, message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (write, mut read) = ws_stream.split();

        let message_forward_handle = spawn(forward_management_socket_messages(
            connection_close.clone(),
            message_rx,
            write,
        ));

        let mut update_rx = self
            .balancer_message_context
            .slot_aggregated_status
            .subscribe_to_updates();

        message_tx
            .send(ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                    name: self.name.clone(),
                    slot_aggregated_status_snapshot: self
                        .balancer_message_context
                        .slot_aggregated_status
                        .make_snapshot(),
                }),
            ))
            .context("The management socket writer stopped before the agent registered")?;

        let agent_ends_connection = loop {
            select! {
                () = connection_close.cancelled() => {
                    info!("Connection close signal received, shutting down");

                    break false;
                }
                () = shutdown.cancelled() => break true,
                Ok(()) = update_rx.changed() => {
                    let status_update = ManagementJsonRpcMessage::Notification(
                        ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                            slot_aggregated_status_snapshot: self
                                .balancer_message_context
                                .slot_aggregated_status
                                .make_snapshot(),
                        }),
                    );

                    if let Err(SendError(_undelivered_status_update)) = message_tx.send(status_update) {
                        info!("The management socket writer stopped, reconnecting");

                        break false;
                    }
                }
                msg = read.next() => {
                    let should_close = match msg {
                        Some(Ok(msg)) => {
                            IncomingMessageContext {
                                balancer_message_context: self.balancer_message_context.clone(),
                                connection_close: connection_close.clone(),
                                message_tx: message_tx.clone(),
                            }
                            .handle_incoming_message(msg);

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

        if let Err(SendError(_undelivered_deregistration)) = message_tx.send(
            ManagementJsonRpcMessage::Notification(ManagementJsonRpcNotification::DeregisterAgent),
        ) {
            info!(
                "The management socket writer already stopped; the balancer deregisters the agent once the connection drops"
            );
        }

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
        let mut ticker = interval(BALANCER_RECONNECT_INTERVAL);

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            select! {
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
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use std::sync::Arc;

    use futures_util::StreamExt as _;
    use serde_json::from_str;
    use serde_json::to_string;
    use tokio::net::TcpListener;
    use tokio::spawn;
    use tokio::sync::mpsc;
    use tokio::sync::oneshot;
    use tokio::sync::watch;
    use tokio::task::JoinHandle;
    use tokio_tungstenite::accept_async;
    use tokio_tungstenite::tungstenite::Bytes;
    use tokio_tungstenite::tungstenite::protocol::Message;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::agent_status::AgentStatus;
    use paddler_messaging::api_path::ApiPath;
    use paddler_messaging::balancer_connection::BalancerConnection;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_messaging::management_socket::agent::message::Message as JsonRpcMessage;
    use paddler_messaging::management_socket::agent::notification::Notification as JsonRpcNotification;
    use paddler_messaging::management_socket::agent::notification_params::set_state_params::SetStateParams;
    use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
    use paddler_messaging::management_socket::agent::request::Request as JsonRpcRequest;
    use paddler_messaging::management_socket::agent::response::Response as JsonRpcResponse;
    use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
    use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
    use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;
    use paddler_messaging::model_metadata::ModelMetadata;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_request_registry::request_delivery::RequestDelivery;
    use paddler_request_registry::request_registry_guard::RequestRegistryGuard;
    use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;

    use super::IncomingMessageContext;
    use super::ManagementSocketClientService;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::balancer_message_context::BalancerMessageContext;
    use crate::pipeline_request::PipelineRequest;

    struct IncomingMessageFixture {
        agent_desired_state_rx: mpsc::UnboundedReceiver<AgentDesiredState>,
        context: IncomingMessageContext,
        pipeline_request_rx: mpsc::UnboundedReceiver<PipelineRequest>,
        message_rx: mpsc::UnboundedReceiver<ManagementJsonRpcMessage>,
    }

    fn incoming_message_fixture() -> IncomingMessageFixture {
        let (agent_desired_state_tx, agent_desired_state_rx) = mpsc::unbounded_channel();
        let (pipeline_request_tx, pipeline_request_rx) = mpsc::unbounded_channel();
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        IncomingMessageFixture {
            agent_desired_state_rx,
            context: IncomingMessageContext {
                balancer_message_context: BalancerMessageContext {
                    agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
                    agent_desired_state_tx,
                    pipeline_request_tx,
                    model_metadata_holder: Arc::new(ModelMetadataHolder::new()),
                    request_stoppers: Arc::default(),
                    slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(2)),
                },
                connection_close: CancellationToken::new(),
                message_tx,
            },
            pipeline_request_rx,
            message_rx,
        }
    }

    fn service_with_socket_url(socket_url: String) -> ManagementSocketClientService {
        let (balancer_connection_tx, _initial_balancer_connection_rx) =
            watch::channel(BalancerConnection::Connecting);

        ManagementSocketClientService {
            balancer_connection_tx,
            balancer_message_context: incoming_message_fixture().context.balancer_message_context,
            name: None,
            socket_url,
        }
    }

    fn raw_prompt_params() -> ContinueFromRawPromptParams {
        ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: NonZeroU32::new(8).unwrap(),
            raw_prompt: "hello".to_owned(),
        }
    }

    fn unconfigured_desired_state() -> AgentDesiredState {
        AgentDesiredState::from(BalancerDesiredState::unconfigured(
            InferenceMode::TextGeneration,
        ))
    }

    fn set_state_message() -> JsonRpcMessage {
        JsonRpcMessage::Notification(JsonRpcNotification::SetState(Box::new(SetStateParams {
            desired_state: unconfigured_desired_state(),
        })))
    }

    struct StalledHandshakeFixture {
        accepted_rx: oneshot::Receiver<()>,
        server: JoinHandle<()>,
        shutdown: CancellationToken,
    }

    fn spawn_stalled_handshake_fixture(listener: TcpListener) -> StalledHandshakeFixture {
        let (accepted_tx, accepted_rx) = oneshot::channel::<()>();
        let shutdown = CancellationToken::new();
        let server_shutdown = shutdown.clone();

        let server = spawn(async move {
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

        let service = service_with_socket_url(format!(
            "ws://{addr}{}",
            ApiPath::agent_socket("test-agent")
        ));
        let shutdown = CancellationToken::new();
        let keep_alive_shutdown = shutdown.clone();
        let keep_alive_handle =
            spawn(async move { service.keep_connection_alive(keep_alive_shutdown).await });

        accepted_rx
            .await
            .expect("the agent's connect must reach the fixture balancer before shutdown");

        shutdown.cancel();

        let keep_alive_result = keep_alive_handle
            .await
            .expect("the keep_connection_alive task must not panic");

        assert!(keep_alive_result.is_ok());

        fixture_shutdown.cancel();
        server
            .await
            .expect("the fixture balancer task must not panic");
    }

    #[tokio::test]
    async fn keep_connection_alive_errors_when_the_balancer_refuses_the_connection() {
        let service = service_with_socket_url(format!(
            "ws://127.0.0.1:1{}",
            ApiPath::agent_socket("test-agent")
        ));
        let shutdown = CancellationToken::new();

        let keep_alive_result = service.keep_connection_alive(shutdown).await;

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

        let service = service_with_socket_url(format!(
            "ws://{addr}{}",
            ApiPath::agent_socket("test-agent")
        ));
        let shutdown = CancellationToken::new();
        let run_shutdown = shutdown.clone();
        let run_handle = spawn(async move { Box::new(service).run(run_shutdown).await });

        accepted_rx
            .await
            .expect("the agent's connect must reach the fixture balancer before shutdown");

        shutdown.cancel();

        let run_result = run_handle.await.expect("the run task must not panic");

        assert!(run_result.is_ok());

        fixture_shutdown.cancel();
        server
            .await
            .expect("the fixture balancer task must not panic");
    }

    #[tokio::test]
    async fn announces_every_status_change_to_the_balancer() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let service = service_with_socket_url(format!(
            "ws://{addr}{}",
            ApiPath::agent_socket("test-agent")
        ));
        let slot_aggregated_status = service
            .balancer_message_context
            .slot_aggregated_status
            .clone();
        let shutdown = CancellationToken::new();
        let keep_alive_shutdown = shutdown.clone();
        let keep_alive_handle =
            spawn(async move { service.keep_connection_alive(keep_alive_shutdown).await });

        let (stream, _peer_addr) = listener.accept().await.unwrap();
        let mut balancer_socket = accept_async(stream).await.unwrap();
        let registration = balancer_socket.next().await.unwrap().unwrap();

        assert!(matches!(
            from_str::<ManagementJsonRpcMessage>(registration.to_text().unwrap()).unwrap(),
            ManagementJsonRpcMessage::Notification(ManagementJsonRpcNotification::RegisterAgent(
                register_agent_params
            )) if register_agent_params.name.is_none()
        ));

        slot_aggregated_status.set_uses_chat_template_override(true);

        let status_update = balancer_socket.next().await.unwrap().unwrap();

        assert!(matches!(
            from_str::<ManagementJsonRpcMessage>(status_update.to_text().unwrap()).unwrap(),
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                    slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot {
                        status: AgentStatus {
                            uses_chat_template_override: true,
                            ..
                        },
                        ..
                    },
                })
            )
        ));

        shutdown.cancel();

        assert!(keep_alive_handle.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn reports_the_balancer_connected_only_while_its_socket_is_open() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let service = service_with_socket_url(format!(
            "ws://{addr}{}",
            ApiPath::agent_socket("test-agent")
        ));
        let mut balancer_connection_rx = service.balancer_connection_tx.subscribe();
        let keep_alive_handle = spawn(async move {
            service
                .keep_connection_alive(CancellationToken::new())
                .await
        });

        let (stream, _peer_addr) = listener.accept().await.unwrap();
        let balancer_socket = accept_async(stream).await.unwrap();

        balancer_connection_rx
            .wait_for(|balancer_connection| *balancer_connection == BalancerConnection::Connected)
            .await
            .unwrap();

        drop(balancer_socket);

        balancer_connection_rx
            .wait_for(|balancer_connection| *balancer_connection == BalancerConnection::Connecting)
            .await
            .unwrap();

        assert!(keep_alive_handle.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn set_state_notification_forwards_desired_state() {
        let mut fixture = incoming_message_fixture();

        fixture
            .context
            .handle_deserialized_message(set_state_message())
            .unwrap();

        assert_eq!(
            fixture.agent_desired_state_rx.try_recv().unwrap(),
            unconfigured_desired_state()
        );
    }

    #[tokio::test]
    async fn set_state_notification_errors_when_receiver_dropped() {
        let fixture = incoming_message_fixture();

        drop(fixture.agent_desired_state_rx);

        assert!(
            fixture
                .context
                .handle_deserialized_message(set_state_message())
                .is_err()
        );
    }

    #[tokio::test]
    async fn stop_responding_to_a_finished_request_is_not_an_error_and_retains_nothing() {
        let fixture = incoming_message_fixture();
        let request_stoppers = fixture
            .context
            .balancer_message_context
            .request_stoppers
            .clone();

        fixture
            .context
            .handle_deserialized_message(JsonRpcMessage::Notification(
                JsonRpcNotification::StopRespondingTo("already_finished".to_owned()),
            ))
            .expect("a stop for a request that already finished must not be an error");

        assert!(
            request_stoppers
                .with_registered("already_finished", |_registered_stopper| ())
                .is_none(),
            "the stop must leave nothing behind; retaining it would leak one entry per cancelled \
             request"
        );
    }

    #[tokio::test]
    async fn stop_responding_to_registered_request_signals_stopper() {
        let fixture = incoming_message_fixture();
        let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<()>();

        let _stopper_guard = RequestRegistryGuard::register(
            &fixture.context.balancer_message_context.request_stoppers,
            "active_request".to_owned(),
            stop_tx,
        )
        .expect("a fresh request id must register");

        fixture
            .context
            .handle_deserialized_message(JsonRpcMessage::Notification(
                JsonRpcNotification::StopRespondingTo("active_request".to_owned()),
            ))
            .unwrap();

        assert_eq!(stop_rx.try_recv(), Ok(()));
    }

    #[tokio::test]
    async fn mismatched_version_notification_is_acknowledged() {
        let mut fixture = incoming_message_fixture();

        fixture
            .context
            .handle_deserialized_message(JsonRpcMessage::Notification(
                JsonRpcNotification::Version(VersionParams {
                    version: "0.0.0-mismatch".to_owned(),
                }),
            ))
            .unwrap();

        assert!(fixture.message_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn get_chat_template_override_without_applicable_state_responds_with_none() {
        let mut fixture = incoming_message_fixture();

        fixture
            .context
            .handle_deserialized_message(JsonRpcMessage::Request(RequestEnvelope {
                id: "req_template".to_owned(),
                request: JsonRpcRequest::GetChatTemplateOverride,
            }))
            .unwrap();

        assert!(matches!(
            fixture.message_rx.try_recv().unwrap(),
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: JsonRpcResponse::ChatTemplateOverride(None),
                ..
            }) if request_id == "req_template"
        ));
    }

    #[tokio::test]
    async fn get_chat_template_override_errors_when_message_receiver_dropped() {
        let fixture = incoming_message_fixture();

        drop(fixture.message_rx);

        assert!(
            fixture
                .context
                .handle_deserialized_message(JsonRpcMessage::Request(RequestEnvelope {
                    id: "req_template".to_owned(),
                    request: JsonRpcRequest::GetChatTemplateOverride,
                }))
                .is_err()
        );
    }

    #[tokio::test]
    async fn get_model_metadata_responds_with_stored_metadata() {
        let mut fixture = incoming_message_fixture();
        let mut metadata = BTreeMap::new();

        metadata.insert("architecture".to_owned(), "llama".to_owned());
        fixture
            .context
            .balancer_message_context
            .model_metadata_holder
            .set_model_metadata(ModelMetadata {
                metadata: metadata.clone(),
            });

        fixture
            .context
            .handle_deserialized_message(JsonRpcMessage::Request(RequestEnvelope {
                id: "req_metadata".to_owned(),
                request: JsonRpcRequest::GetModelMetadata,
            }))
            .unwrap();

        assert!(matches!(
            fixture.message_rx.try_recv().unwrap(),
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                response: JsonRpcResponse::ModelMetadata(Some(returned_metadata)),
                ..
            }) if returned_metadata.metadata == metadata
        ));
    }

    #[tokio::test]
    async fn get_model_metadata_errors_when_message_receiver_dropped() {
        let fixture = incoming_message_fixture();

        drop(fixture.message_rx);

        assert!(
            fixture
                .context
                .handle_deserialized_message(JsonRpcMessage::Request(RequestEnvelope {
                    id: "req_metadata".to_owned(),
                    request: JsonRpcRequest::GetModelMetadata,
                }))
                .is_err()
        );
    }

    #[tokio::test]
    async fn text_message_dispatches_deserialized_set_state() {
        let mut fixture = incoming_message_fixture();

        fixture.context.handle_incoming_message(Message::Text(
            to_string(&set_state_message()).unwrap().into(),
        ));

        assert_eq!(
            fixture.agent_desired_state_rx.recv().await.unwrap(),
            unconfigured_desired_state()
        );
    }

    #[tokio::test]
    async fn undecodable_text_message_closes_the_connection() {
        let fixture = incoming_message_fixture();
        let connection_close = fixture.context.connection_close.clone();

        fixture
            .context
            .handle_incoming_message(Message::Text("not json".into()));

        assert!(connection_close.is_cancelled());
    }

    #[tokio::test]
    async fn binary_message_closes_the_connection() {
        let fixture = incoming_message_fixture();
        let connection_close = fixture.context.connection_close.clone();

        fixture
            .context
            .handle_incoming_message(Message::Binary(Bytes::from_static(b"\x00\x01")));

        assert!(connection_close.is_cancelled());
    }

    #[tokio::test]
    async fn generate_responses_registers_the_stopper_before_it_returns() {
        let mut fixture = incoming_message_fixture();
        let request_stoppers = fixture
            .context
            .balancer_message_context
            .request_stoppers
            .clone();

        fixture
            .context
            .generate_responses::<_, GeneratedTokenResult>(
                "req_generate".to_owned(),
                raw_prompt_params(),
            )
            .expect("the request must be accepted");

        let dispatched_request = fixture
            .pipeline_request_rx
            .try_recv()
            .expect("the request must be dispatched to the arbiter");

        assert!(matches!(
            &dispatched_request,
            PipelineRequest::ContinueFromRawPrompt(request)
                if request.params.raw_prompt == "hello"
        ));
        assert_eq!(
            request_stoppers.send_to("req_generate", ()),
            RequestDelivery::Delivered,
            "the stopper must be registered before generate_responses returns, so a stop arriving \
             on the next frame cannot be lost"
        );
    }

    #[tokio::test]
    async fn generate_responses_errors_when_request_receiver_dropped() {
        let fixture = incoming_message_fixture();

        drop(fixture.pipeline_request_rx);

        assert!(
            fixture
                .context
                .generate_responses::<_, GeneratedTokenResult>(
                    "req_generate".to_owned(),
                    raw_prompt_params(),
                )
                .is_err()
        );
    }

    #[tokio::test]
    async fn generate_responses_errors_when_stopper_already_registered() {
        let fixture = incoming_message_fixture();
        let (existing_stop_tx, _existing_stop_rx) = mpsc::unbounded_channel::<()>();

        let _existing_stopper_guard = RequestRegistryGuard::register(
            &fixture.context.balancer_message_context.request_stoppers,
            "req_generate".to_owned(),
            existing_stop_tx,
        )
        .expect("a fresh request id must register");

        assert!(
            fixture
                .context
                .generate_responses::<_, GeneratedTokenResult>(
                    "req_generate".to_owned(),
                    raw_prompt_params(),
                )
                .is_err()
        );
    }
}
