use std::sync::Arc;

use actix_web::rt;
use actix_web::web::Bytes;
use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use futures_util::SinkExt as _;
use futures_util::StreamExt;
use log::debug;
use log::error;
use log::info;
use log::warn;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::agent_desired_state::AgentDesiredState;
use crate::agent::receive_stream_stopper_collection::ReceiveStreamStopperCollection;
use crate::agent::jsonrpc::Message as JsonRpcMessage;
use crate::agent::jsonrpc::Notification as JsonRpcNotification;
use crate::agent::jsonrpc::Request as JsonRpcRequest;
use crate::agent::jsonrpc::Response as JsonRpcResponse;
use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::agent::jsonrpc::notification_params::SetStateParams;
use crate::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::agent::jsonrpc::notification_params::VersionParams;
use crate::slot_aggregated_status::SlotAggregatedStatus;
use crate::agent::from_request_params::FromRequestParams;
use crate::balancer::management_service::http_route::api::ws_agent_socket::jsonrpc::Message as ManagementJsonRpcMessage;
use crate::balancer::management_service::http_route::api::ws_agent_socket::jsonrpc::Notification as ManagementJsonRpcNotification;
use crate::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::balancer::management_service::http_route::api::ws_agent_socket::jsonrpc::notification_params::RegisterAgentParams;
use crate::balancer::management_service::http_route::api::ws_agent_socket::jsonrpc::notification_params::UpdateAgentStatusParams;
use crate::jsonrpc::Error as JsonRpcError;
use crate::jsonrpc::ResponseEnvelope;
use crate::jsonrpc::RequestEnvelope;
use crate::produces_snapshot::ProducesSnapshot;
use crate::jsonrpc::ErrorEnvelope;
use crate::service::Service;
use crate::agent::model_metadata_holder::ModelMetadataHolder;

struct IncomingMessageContext {
    agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    agent_desired_state_tx: mpsc::UnboundedSender<AgentDesiredState>,
    connection_close_tx: broadcast::Sender<()>,
    continue_from_conversation_history_request_tx:
        mpsc::UnboundedSender<ContinueFromConversationHistoryRequest>,
    continue_from_raw_prompt_request_tx: mpsc::UnboundedSender<ContinueFromRawPromptRequest>,
    generate_embedding_batch_request_tx: mpsc::UnboundedSender<GenerateEmbeddingBatchRequest>,
    model_metadata_holder: Arc<ModelMetadataHolder>,
    receive_stream_stopper_collection: Arc<ReceiveStreamStopperCollection>,
    message_tx: mpsc::UnboundedSender<ManagementJsonRpcMessage>,
}

pub struct ManagementSocketClientService {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_desired_state_tx: mpsc::UnboundedSender<AgentDesiredState>,
    pub continue_from_conversation_history_request_tx:
        mpsc::UnboundedSender<ContinueFromConversationHistoryRequest>,
    pub continue_from_raw_prompt_request_tx: mpsc::UnboundedSender<ContinueFromRawPromptRequest>,
    pub generate_embedding_batch_request_tx: mpsc::UnboundedSender<GenerateEmbeddingBatchRequest>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub name: Option<String>,
    pub receive_stream_stopper_collection: Arc<ReceiveStreamStopperCollection>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    pub socket_url: String,
}

impl ManagementSocketClientService {
    async fn generate_responses<TRequest: FromRequestParams + 'static>(
        connection_close_tx: broadcast::Sender<()>,
        id: String,
        message_tx: mpsc::UnboundedSender<ManagementJsonRpcMessage>,
        request_params: TRequest::RequestParams,
        receive_stream_stopper_collection: Arc<ReceiveStreamStopperCollection>,
        request_tx: mpsc::UnboundedSender<TRequest>,
    ) -> Result<()> {
        let (response_tx, mut response_rx) = mpsc::unbounded_channel::<TRequest::Response>();
        let (stop_tx, stop_rx) = mpsc::unbounded_channel::<()>();

        let _guard = receive_stream_stopper_collection
            .register_stopper_with_guard(id.clone(), stop_tx)
            .context(format!("Failed to register stopper for request: {id}"))?;

        request_tx.send(TRequest::from_request_params(
            request_params,
            response_tx,
            stop_rx,
        ))?;

        let mut connection_close_rx = connection_close_tx.subscribe();

        loop {
            tokio::select! {
                _ = connection_close_rx.recv() => break,
                response = response_rx.recv() => {
                    match response {
                        Some(response) => {
                            message_tx.send(
                                ManagementJsonRpcMessage::Response(
                                    ResponseEnvelope {
                                        request_id: id.clone(),
                                        response: response.into(),
                                    }
                                ),
                            )?;
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_deserialized_message(
        IncomingMessageContext {
            agent_applicable_state_holder,
            agent_desired_state_tx,
            connection_close_tx,
            continue_from_conversation_history_request_tx,
            continue_from_raw_prompt_request_tx,
            generate_embedding_batch_request_tx,
            message_tx,
            model_metadata_holder,
            receive_stream_stopper_collection,
        }: IncomingMessageContext,
        deserialized_message: JsonRpcMessage,
    ) -> Result<()> {
        match deserialized_message {
            JsonRpcMessage::Error(ErrorEnvelope {
                request_id,
                error: JsonRpcError { code, description },
            }) => {
                error!(
                    "Received error from server: code: {code}, description: {description:?}, request_id: {request_id:?}"
                );

                Ok(())
            }
            JsonRpcMessage::Notification(JsonRpcNotification::SetState(SetStateParams {
                desired_state,
            })) => {
                agent_desired_state_tx.send(desired_state)?;

                Ok(())
            }
            JsonRpcMessage::Notification(JsonRpcNotification::StopRespondingTo(request_id)) => {
                debug!("Received StopGeneratingTokens notification for request ID: {request_id:?}");
                receive_stream_stopper_collection
                    .stop(request_id.clone())
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
            }) => {
                Self::generate_responses(
                    connection_close_tx,
                    id,
                    message_tx,
                    continue_from_conversation_history_params,
                    receive_stream_stopper_collection,
                    continue_from_conversation_history_request_tx,
                )
                .await
            }
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::ContinueFromRawPrompt(generate_tokens_params),
            }) => {
                Self::generate_responses(
                    connection_close_tx,
                    id,
                    message_tx,
                    generate_tokens_params,
                    receive_stream_stopper_collection,
                    continue_from_raw_prompt_request_tx,
                )
                .await
            }
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GenerateEmbeddingBatch(generate_embedding_batch_params),
            }) => {
                Self::generate_responses(
                    connection_close_tx,
                    id,
                    message_tx,
                    generate_embedding_batch_params,
                    receive_stream_stopper_collection,
                    generate_embedding_batch_request_tx,
                )
                .await
            }
            JsonRpcMessage::Request(RequestEnvelope {
                id,
                request: JsonRpcRequest::GetChatTemplateOverride,
            }) => Ok(
                message_tx.send(ManagementJsonRpcMessage::Response(ResponseEnvelope {
                    request_id: id.clone(),
                    response: JsonRpcResponse::ChatTemplateOverride(
                        if let Some(agent_applicable_state) =
                            agent_applicable_state_holder.get_agent_applicable_state()
                        {
                            agent_applicable_state.chat_template_override.clone()
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
                    request_id: id.clone(),
                    response: JsonRpcResponse::ModelMetadata(
                        model_metadata_holder.get_model_metadata(),
                    ),
                }))?,
            ),
        }
    }

    async fn handle_incoming_message(
        incoming_message_context: IncomingMessageContext,
        msg: Message,
        pong_tx: mpsc::UnboundedSender<Bytes>,
    ) -> Result<()> {
        match msg {
            Message::Text(text) => {
                let mut connection_close_rx =
                    incoming_message_context.connection_close_tx.subscribe();

                rt::spawn(async move {
                    tokio::select! {
                        _ = connection_close_rx.recv() => {
                            info!("Connection close signal received, shutting down");
                        }
                        result = Self::handle_deserialized_message(
                            incoming_message_context,
                            match serde_json::from_str::<JsonRpcMessage>(&text).context(format!("Failed to parse JSON-RPC message: {text}")) {
                                Ok(message) => message,
                                Err(err) => {
                                    error!("Failed to deserialize message: {err}");

                                    return;
                                }
                            },
                        ) => if let Err(err) = result {
                            error!("Error handling incoming message: {err}");
                        }
                    }
                });

                Ok(())
            }
            Message::Binary(_) => {
                error!("Received binary message, which is not expected");

                Ok(())
            }
            Message::Close(_) => {
                info!("Connection closed by server");

                Ok(())
            }
            Message::Frame(_) => {
                error!("Received a frame message, which is not expected");

                Ok(())
            }
            Message::Ping(payload) => Ok(pong_tx.send(payload)?),
            Message::Pong(_) => {
                // Pong received, no action needed
                Ok(())
            }
        }
    }

    async fn keep_connection_alive(&self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        info!("Connecting to management server at {}", self.socket_url);

        let (ws_stream, _response) = connect_async(self.socket_url.clone()).await?;

        info!("Connected to management server");

        let (connection_close_tx, mut connection_close_rx) = broadcast::channel::<()>(1);
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<ManagementJsonRpcMessage>();
        let (pong_tx, mut pong_rx) = mpsc::unbounded_channel::<Bytes>();
        let (mut write, mut read) = ws_stream.split();

        let mut connection_close_rx_resubscribed = connection_close_rx.resubscribe();
        let mut shutdown_resubscribed = shutdown.resubscribe();

        let message_forward_handle = rt::spawn(async move {
            loop {
                tokio::select! {
                    _ = connection_close_rx_resubscribed.recv() => {
                        break;
                    }
                    _ = shutdown_resubscribed.recv() => {
                        info!("Shutdown signal received, deregistering agent");

                        write.send(Message::Text(match serde_json::to_string(
                            &ManagementJsonRpcMessage::Notification(
                                ManagementJsonRpcNotification::DeregisterAgent,
                            )
                        ) {
                            Ok(serialized_message) => serialized_message.into(),
                            Err(err) => {
                                error!("Failed to serialize deregister agent notification: {err}");
                                return;
                            }
                        })).await.unwrap_or_else(|err| {
                            error!("Failed to send deregister agent notification: {err}");
                        });

                        break;
                    }
                    message = message_rx.recv() => {
                        match message {
                            Some(msg) => {
                                match serde_json::to_string(&msg) {
                                    Ok(serialized_message) => {
                                        let message = Message::Text(serialized_message.into());

                                        if let Err(err) = write.send(message).await {
                                            error!("Failed to send message: {err}");
                                            break;
                                        }
                                    },
                                    Err(err) => {
                                        error!("Failed to serialize message: {err}");
                                    }
                                };
                            }
                            None => break,
                        }
                    }
                    payload = pong_rx.recv() => {
                        match payload {
                            Some(payload) => {
                                write.send(Message::Pong(payload)).await.unwrap_or_else(|err| {
                                    error!("Failed to send pong message: {err}");
                                });
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        match self.slot_aggregated_status.make_snapshot() {
            Ok(slot_aggregated_status_snapshot) => {
                message_tx
                    .send(ManagementJsonRpcMessage::Notification(
                        ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                            name: self.name.clone(),
                            slot_aggregated_status_snapshot,
                        }),
                    ))
                    .unwrap_or_else(|err| {
                        error!("Failed to send register agent notification: {err}");
                    });
            }
            Err(err) => {
                error!("Failed to create slot aggregated status snapshot: {err}");

                return Err(err);
            }
        }

        let do_send_status_update = || match self.slot_aggregated_status.make_snapshot() {
            Ok(slot_aggregated_status_snapshot) => {
                message_tx
                    .send(ManagementJsonRpcMessage::Notification(
                        ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                            slot_aggregated_status_snapshot,
                        }),
                    ))
                    .unwrap_or_else(|err| {
                        error!("Failed to send status update notification: {err}");
                    });
            }
            Err(err) => error!("Failed to create slot aggregated status snapshot: {err}"),
        };

        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = connection_close_rx.recv() => {
                    info!("Connection close signal received, shutting down");

                    break;
                }
                _ = shutdown.recv() => break,
                _ = self.slot_aggregated_status.update_notifier.notified() => do_send_status_update(),
                _ = ticker.tick() => do_send_status_update(),
                msg = read.next() => {
                    let should_close = match msg {
                        Some(Ok(msg)) => {
                            if let Err(err) = Self::handle_incoming_message(
                                    IncomingMessageContext {
                                        agent_applicable_state_holder: self.agent_applicable_state_holder.clone(),
                                        agent_desired_state_tx: self.agent_desired_state_tx.clone(),
                                        connection_close_tx: connection_close_tx.clone(),
                                        continue_from_conversation_history_request_tx: self.continue_from_conversation_history_request_tx.clone(),
                                        continue_from_raw_prompt_request_tx: self.continue_from_raw_prompt_request_tx.clone(),
                                        generate_embedding_batch_request_tx: self.generate_embedding_batch_request_tx.clone(),
                                        model_metadata_holder: self.model_metadata_holder.clone(),
                                        receive_stream_stopper_collection: self.receive_stream_stopper_collection.clone(),
                                        message_tx: message_tx.clone(),
                                    },
                                    msg,
                                    pong_tx.clone(),
                                )
                                .await
                                .context("Failed to handle incoming message")
                            {
                                error!("Error handling incoming message: {err}");
                            }

                            false
                        }
                        Some(Err(err)) => {
                            error!("Error reading message: {err}");

                            true
                        }
                        None => true,
                    };

                    if should_close {
                        if let Err(err) = connection_close_tx.send(()) {
                            error!("Failed to send connection close signal: {err}");
                        }

                        break;
                    }
                }
            }
        }

        message_forward_handle
            .await
            .context("Failed to join message forwarding task")?;

        Ok(())
    }
}

#[async_trait]
impl Service for ManagementSocketClientService {
    fn name(&self) -> &'static str {
        "agent::management_socket_client_service"
    }

    async fn run(&mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = shutdown.recv() => break Ok(()),
                _ = ticker.tick() => {
                    match self.keep_connection_alive(shutdown.resubscribe()).await {
                        Err(err) => {
                            error!("Failed to keep the connection alive: {err:?}");
                        }
                        Ok(()) => {
                            info!("Gracefully closed connection to management server");
                        }
                    }
                }
            }
        }
    }
}
