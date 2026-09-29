mod agent_socket_controller_context;

use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::AtomicI32;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::get;
use actix_web::rt;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use actix_ws::CloseCode;
use actix_ws::CloseReason;
use actix_ws::Session;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use log::info;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use self::agent_socket_controller_context::AgentSocketControllerContext;
use crate::agent_controller::AgentController;
use crate::agent_desired_state_notification::agent_desired_state_notification;
use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_controller_registration::AgentControllerRegistration;
use crate::agent_status::AgentStatus;
use crate::agent_controller_update_result::AgentControllerUpdateResult;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::continuation_decision::ContinuationDecision;
use crate::continuation_stop_parameters::ContinuationStopParameters;
use crate::controls_session::ControlsSession as _;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::embedding_sender_collection::EmbeddingSenderCollection;
use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::management_service::app_data::AppData;
use crate::manages_senders::ManagesSenders as _;
use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::websocket_session_controller::WebSocketSessionController;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;

async fn send_to_agent(
    websocket_session_controller: &mut WebSocketSessionController<AgentJsonRpcMessage>,
    message: AgentJsonRpcMessage,
) {
    if let Err(err) = websocket_session_controller.send_response(message).await {
        error!("Error sending response: {err}");
    }
}

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
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        mut websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> ContinuationDecision {
        match deserialized_message {
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::DeregisterAgent,
            ) => {
                connection_close.cancel();

                return ContinuationDecision::Stop(ContinuationStopParameters {
                    close_reason: None,
                });
            }
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                    name,
                    slot_aggregated_status_snapshot,
                }),
            ) => {
                let (agent_message_tx, mut agent_message_rx) =
                    mpsc::unbounded_channel::<AgentJsonRpcMessage>();
                let agent_controller = Arc::new(AgentController {
                    agent_message_tx,
                    chat_template_override_sender_collection: context
                        .chat_template_override_sender_collection
                        .clone(),
                    connection_close: connection_close.clone(),
                    embedding_sender_collection: context.embedding_sender_collection.clone(),
                    generate_tokens_sender_collection: context
                        .generate_tokens_sender_collection
                        .clone(),
                    id: context.agent_id.clone(),
                    model_metadata_sender_collection: context
                        .model_metadata_sender_collection
                        .clone(),
                    name,
                    slots_processing: AtomicValue::<AtomicI32>::new(
                        slot_aggregated_status_snapshot.slots_processing,
                    ),
                    status: RwLock::new(AgentStatus::from(slot_aggregated_status_snapshot)),
                });

                let registered_agent_controller_guard = match context
                    .agent_controller_pool
                    .register_agent_controller(context.agent_id.clone(), agent_controller)
                {
                    AgentControllerRegistration::DuplicateAgentId => {
                        error!(
                            "Rejecting a second registration of agent {}",
                            context.agent_id
                        );

                        return ContinuationDecision::Stop(ContinuationStopParameters {
                            close_reason: Some(CloseReason {
                                code: CloseCode::Policy,
                                description: Some(format!(
                                    "Agent {} is already registered",
                                    context.agent_id
                                )),
                            }),
                        });
                    }
                    AgentControllerRegistration::Registered(registered_agent_controller_guard) => {
                        registered_agent_controller_guard
                    }
                };

                info!("Registered agent: {}", context.agent_id);

                let agent_desired_state = agent_desired_state_notification(
                    context
                        .balancer_applicable_state_holder
                        .get_agent_desired_state(),
                );
                let forwarder_close = connection_close.clone();

                rt::spawn(async move {
                    let _registered_agent_controller_guard = registered_agent_controller_guard;

                    send_to_agent(&mut websocket_session_controller, agent_desired_state).await;

                    loop {
                        tokio::select! {
                            () = forwarder_close.cancelled() => {
                                break;
                            }
                            Some(message) = agent_message_rx.recv() => {
                                send_to_agent(&mut websocket_session_controller, message).await;
                            }
                        }
                    }
                });

                ContinuationDecision::Continue
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
                            context.agent_controller_pool.signal_update();
                        }
                    }
                } else {
                    error!("Agent controller not found for agent: {}", context.agent_id);
                }

                ContinuationDecision::Continue
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::ChatTemplateOverride(chat_template_override),
                ..
            }) => {
                context
                    .chat_template_override_sender_collection
                    .forward_response_safe(request_id, chat_template_override)
                    .await;

                ContinuationDecision::Continue
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::Embedding(embedding_result),
                ..
            }) => {
                context
                    .embedding_sender_collection
                    .forward_response_safe(request_id, embedding_result)
                    .await;

                ContinuationDecision::Continue
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::GeneratedToken(generated_token_envelope),
                ..
            }) => {
                context
                    .generate_tokens_sender_collection
                    .forward_response_safe(request_id, generated_token_envelope)
                    .await;

                ContinuationDecision::Continue
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::ModelMetadata(model_metadata),
                ..
            }) => {
                context
                    .model_metadata_sender_collection
                    .forward_response_safe(request_id, model_metadata)
                    .await;

                ContinuationDecision::Continue
            }
        }
    }

    async fn on_connection_start(
        _connection_close: CancellationToken,
        _context: Arc<Self::Context>,
        session: &mut Session,
    ) {
        if let Err(err) = WebSocketSessionController::new(session.clone())
            .send_response(AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::Version(VersionParams {
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                }),
            ))
            .await
        {
            error!("Error sending version: {err:?}");
        }
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

    agent_socket_controller.respond(payload, req, app_data.shutdown.clone())
}
