mod agent_socket_controller_context;
pub mod jsonrpc;

use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicUsize;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::get;
use actix_web::rt;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use actix_ws::Session;
use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use log::info;
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use self::agent_socket_controller_context::AgentSocketControllerContext;
use self::jsonrpc::Message as ManagementJsonRpcMessage;
use self::jsonrpc::Notification as ManagementJsonRpcNotification;
use self::jsonrpc::notification_params::RegisterAgentParams;
use self::jsonrpc::notification_params::UpdateAgentStatusParams;
use crate::agent::jsonrpc::Message as AgentJsonRpcMessage;
use crate::agent::jsonrpc::Notification as AgentJsonRpcNotification;
use crate::agent::jsonrpc::Response as AgentJsonRpcResponse;
use crate::agent::jsonrpc::notification_params::VersionParams;
use crate::atomic_value::AtomicValue;
use crate::balancer::agent_controller::AgentController;
use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::agent_controller_update_result::AgentControllerUpdateResult;
use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::balancer::management_service::app_data::AppData;
use crate::balancer::manages_senders::ManagesSenders as _;
use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::controls_session::ControlsSession as _;
use crate::controls_websocket_endpoint::ContinuationDecision;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::jsonrpc::ResponseEnvelope;
use crate::sets_desired_state::SetsDesiredState as _;
use crate::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use crate::websocket_session_controller::WebSocketSessionController;

pub fn register(cfg: &mut ServiceConfig) {
    cfg.service(respond);
}

struct AgentSocketController {
    agent_controller_pool: Arc<AgentControllerPool>,
    agent_id: String,
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
}

#[async_trait]
impl ControlsWebSocketEndpoint for AgentSocketController {
    type Context = AgentSocketControllerContext;
    type IncomingMessage = ManagementJsonRpcMessage;
    type OutgoingMessage = AgentJsonRpcMessage;

    fn create_context(&self) -> Self::Context {
        AgentSocketControllerContext {
            agent_controller_pool: self.agent_controller_pool.clone(),
            agent_id: self.agent_id.clone(),
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            chat_template_override_sender_collection: self
                .chat_template_override_sender_collection
                .clone(),
            embedding_sender_collection: self.embedding_sender_collection.clone(),
            generate_tokens_sender_collection: self.generate_tokens_sender_collection.clone(),
            model_metadata_sender_collection: self.model_metadata_sender_collection.clone(),
        }
    }

    async fn handle_deserialized_message(
        connection_close_tx: broadcast::Sender<()>,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        mut websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> Result<ContinuationDecision> {
        match deserialized_message {
            ManagementJsonRpcMessage::Error(err) => {
                error!("Received error message: {err:?}");

                Ok(ContinuationDecision::Continue)
            }
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::DeregisterAgent,
            ) => {
                connection_close_tx.send(())?;

                return Ok(ContinuationDecision::Stop);
            }
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                    name,
                    slot_aggregated_status_snapshot:
                        SlotAggregatedStatusSnapshot {
                            desired_slots_total,
                            download_current,
                            download_filename,
                            download_total,
                            issues,
                            model_path,
                            slots_processing,
                            slots_total,
                            state_application_status,
                            uses_chat_template_override,
                            version,
                        },
                }),
            ) => {
                let (agent_message_tx, mut agent_message_rx) =
                    mpsc::unbounded_channel::<AgentJsonRpcMessage>();
                let agent_controller = Arc::new(AgentController {
                    agent_message_tx,
                    chat_template_override_sender_collection: context
                        .chat_template_override_sender_collection
                        .clone(),
                    connection_close_rx: connection_close_tx.subscribe(),
                    desired_slots_total: AtomicValue::<AtomicI32>::new(desired_slots_total),
                    download_current: AtomicValue::<AtomicUsize>::new(download_current),
                    download_filename: RwLock::new(download_filename),
                    download_total: AtomicValue::<AtomicUsize>::new(download_total),
                    embedding_sender_collection: context.embedding_sender_collection.clone(),
                    generate_tokens_sender_collection: context
                        .generate_tokens_sender_collection
                        .clone(),
                    model_metadata_sender_collection: context
                        .model_metadata_sender_collection
                        .clone(),
                    id: context.agent_id.clone(),
                    issues: RwLock::new(issues),
                    model_path: RwLock::new(model_path),
                    name,
                    newest_update_version: AtomicValue::<AtomicI32>::new(version),
                    slots_processing: AtomicValue::<AtomicI32>::new(slots_processing),
                    slots_total: AtomicValue::<AtomicI32>::new(slots_total),
                    state_application_status_code: AtomicValue::<AtomicI32>::new(
                        state_application_status as i32,
                    ),
                    uses_chat_template_override: AtomicValue::<AtomicBool>::new(
                        uses_chat_template_override,
                    ),
                });

                context
                    .agent_controller_pool
                    .register_agent_controller(context.agent_id.clone(), agent_controller.clone())
                    .context("Unable to register agent controller")?;

                if let Some(desired_state) = context
                    .balancer_applicable_state_holder
                    .get_agent_desired_state()
                {
                    agent_controller
                        .set_desired_state(desired_state)
                        .await
                        .context("Unable to set desired state")?;
                }

                info!("Registered agent: {}", context.agent_id);

                let mut shutdown_tx_resubscribed = connection_close_tx.subscribe();

                rt::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = shutdown_tx_resubscribed.recv() => {
                                break;
                            }
                            result = agent_message_rx.recv() => {
                                match result {
                                    Some(message) => {
                                        websocket_session_controller
                                            .send_response(message)
                                            .await
                                            .unwrap_or_else(|err| {
                                                error!("Error sending response: {err}");
                                            });
                                    }
                                    None => {
                                        info!("Session channel closed for agent: {}", context.agent_id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                Ok(ContinuationDecision::Continue)
            }
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::UpdateAgentStatus(UpdateAgentStatusParams {
                    slot_aggregated_status_snapshot,
                }),
            ) => {
                if let Some(agent_controller) = context
                    .agent_controller_pool
                    .get_agent_controller(&context.agent_id)
                {
                    match agent_controller.update_from_slot_aggregated_status_snapshot(
                        slot_aggregated_status_snapshot,
                    ) {
                        AgentControllerUpdateResult::NoMeaningfulChanges => {}
                        AgentControllerUpdateResult::Updated => {
                            context
                                .agent_controller_pool
                                .update_notifier
                                .notify_waiters();
                        }
                    }
                } else {
                    error!("Agent controller not found for agent: {}", context.agent_id);
                }

                Ok(ContinuationDecision::Continue)
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::ChatTemplateOverride(chat_template_override),
            }) => {
                context
                    .chat_template_override_sender_collection
                    .forward_response_safe(request_id, chat_template_override)
                    .await;

                Ok(ContinuationDecision::Continue)
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::Embedding(embedding_result),
            }) => {
                context
                    .embedding_sender_collection
                    .forward_response_safe(request_id, embedding_result)
                    .await;

                Ok(ContinuationDecision::Continue)
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::GeneratedToken(generated_token_envelope),
            }) => {
                context
                    .generate_tokens_sender_collection
                    .forward_response_safe(request_id, generated_token_envelope)
                    .await;

                Ok(ContinuationDecision::Continue)
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::ModelMetadata(model_metadata),
            }) => {
                context
                    .model_metadata_sender_collection
                    .forward_response_safe(request_id, model_metadata)
                    .await;

                Ok(ContinuationDecision::Continue)
            }
        }
    }

    async fn on_connection_start(
        _context: Arc<Self::Context>,
        session: &mut Session,
    ) -> Result<ContinuationDecision> {
        if let Err(err) = session
            .text(serde_json::to_string(&AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::Version(VersionParams {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                }),
            ))?)
            .await
        {
            error!("Error sending version: {err:?}");

            return Ok(ContinuationDecision::Stop);
        }

        Ok(ContinuationDecision::Continue)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathParams {
    agent_id: String,
}

#[get("/api/v1/agent_socket/{agent_id}")]
async fn respond(
    app_data: Data<AppData>,
    path_params: Path<PathParams>,
    payload: Payload,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let agent_socket_controller = AgentSocketController {
        agent_controller_pool: app_data.agent_controller_pool.clone(),
        agent_id: path_params.agent_id.clone(),
        balancer_applicable_state_holder: app_data.balancer_applicable_state_holder.clone(),
        chat_template_override_sender_collection: app_data
            .chat_template_override_sender_collection
            .clone(),
        embedding_sender_collection: app_data.embedding_sender_collection.clone(),
        generate_tokens_sender_collection: app_data.generate_tokens_sender_collection.clone(),
        model_metadata_sender_collection: app_data.model_metadata_sender_collection.clone(),
    };

    agent_socket_controller.respond(payload, req)
}
