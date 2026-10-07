mod agent_socket_controller_context;
mod agent_socket_registration;

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::rt;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use actix_web::web::get;
use actix_ws::CloseCode;
use actix_ws::CloseReason;
use actix_ws::Session;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use log::info;
use log::warn;
use parking_lot::RwLock;
use serde_json::Error as SerdeJsonError;
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;
use paddler_request_registry::request_delivery::RequestDelivery;

use self::agent_socket_controller_context::AgentSocketControllerContext;
use self::agent_socket_registration::AgentSocketRegistration;
use crate::agent_controller::AgentController;
use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_controller_registration::AgentControllerRegistration;
use crate::agent_controller_update_result::AgentControllerUpdateResult;
use crate::agent_desired_state_notification::agent_desired_state_notification;
use crate::agent_id_path_params::AgentIdPathParams;
use crate::agent_response_senders::AgentResponseSenders;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::continuation_decision::ContinuationDecision;
use crate::continuation_stop_parameters::ContinuationStopParameters;
use crate::controls_session::ControlsSession as _;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::management_service::app_data::AppData;
use crate::response_senders::ResponseSenders;
use crate::websocket_session_controller::WebSocketSessionController;

fn forward_agent_response<TResponse>(
    response_senders: &ResponseSenders<TResponse>,
    request_id: &str,
    response: TResponse,
) {
    match response_senders.send_to(request_id, response) {
        RequestDelivery::Delivered => {}
        RequestDelivery::ReceiverDropped => {
            warn!("Dropped an agent response for request {request_id:?}: its receiver went away");
        }
        RequestDelivery::RequestNotRegistered => {
            warn!("Dropped an agent response for unknown request {request_id:?}");
        }
    }
}

async fn send_to_agent(
    websocket_session_controller: &mut WebSocketSessionController<AgentJsonRpcMessage>,
    message: AgentJsonRpcMessage,
) {
    if let Err(err) = websocket_session_controller.send_response(message).await {
        error!("Error sending response: {err}");
    }
}

async fn respond(
    app_data: Data<AppData>,
    path_params: Path<AgentIdPathParams>,
    payload: Payload,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let agent_socket_controller = AgentSocketController {
        agent_controller_pool: app_data.agent_controller_pool.clone(),
        agent_id: path_params.agent_id.clone(),
        balancer_applicable_state_holder: app_data.balancer_applicable_state_holder.clone(),
        agent_response_senders: app_data.agent_response_senders.clone(),
    };

    agent_socket_controller.respond(payload, req, app_data.shutdown.clone())
}

struct AgentSocketController {
    agent_controller_pool: Arc<AgentControllerPool>,
    agent_id: String,
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    agent_response_senders: AgentResponseSenders,
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
            agent_response_senders: self.agent_response_senders.clone(),
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            registration: RwLock::new(AgentSocketRegistration::AwaitingRegistration),
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
                    agent_response_senders: context.agent_response_senders.clone(),
                    connection_close: connection_close.clone(),
                    id: context.agent_id.clone(),
                    name,
                    reported_status: RwLock::new(slot_aggregated_status_snapshot),
                    slots_processing: AtomicValue::<AtomicU64>::new(0),
                });

                let registered_agent_controller_guard = match context
                    .agent_controller_pool
                    .register_agent_controller(agent_controller.clone())
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

                *context.registration.write() =
                    AgentSocketRegistration::Registered(agent_controller);

                info!("Registered agent: {}", context.agent_id);

                let agent_desired_state = agent_desired_state_notification(
                    context
                        .balancer_applicable_state_holder
                        .get_agent_desired_state(),
                );
                let forwarder_close = connection_close;

                rt::spawn(async move {
                    let _registered_agent_controller_guard = registered_agent_controller_guard;

                    send_to_agent(&mut websocket_session_controller, agent_desired_state).await;

                    loop {
                        select! {
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
            ) => match &*context.registration.read() {
                AgentSocketRegistration::AwaitingRegistration => {
                    error!(
                        "Rejecting a status update from agent {} before its registration",
                        context.agent_id
                    );

                    ContinuationDecision::Stop(ContinuationStopParameters {
                        close_reason: Some(CloseReason {
                            code: CloseCode::Policy,
                            description: Some(format!(
                                "Agent {} sent its status before registering",
                                context.agent_id
                            )),
                        }),
                    })
                }
                AgentSocketRegistration::Registered(agent_controller) => {
                    match agent_controller.update_from_slot_aggregated_status_snapshot(
                        slot_aggregated_status_snapshot,
                    ) {
                        AgentControllerUpdateResult::NoMeaningfulChanges => {}
                        AgentControllerUpdateResult::Updated => {
                            context.agent_controller_pool.signal_update();
                        }
                    }

                    ContinuationDecision::Continue
                }
            },
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::ChatTemplateOverride(chat_template_override),
                ..
            }) => {
                forward_agent_response(
                    &context.agent_response_senders.chat_template_override,
                    &request_id,
                    chat_template_override,
                );

                ContinuationDecision::Continue
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::Decision(decision_result),
                ..
            }) => {
                forward_agent_response(
                    &context.agent_response_senders.decision,
                    &request_id,
                    decision_result,
                );

                ContinuationDecision::Continue
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::Embedding(embedding_result),
                ..
            }) => {
                forward_agent_response(
                    &context.agent_response_senders.embedding,
                    &request_id,
                    embedding_result,
                );

                ContinuationDecision::Continue
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::GeneratedToken(generated_token_envelope),
                ..
            }) => {
                forward_agent_response(
                    &context.agent_response_senders.generated_tokens,
                    &request_id,
                    generated_token_envelope,
                );

                ContinuationDecision::Continue
            }
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response: AgentJsonRpcResponse::ModelMetadata(model_metadata),
                ..
            }) => {
                forward_agent_response(
                    &context.agent_response_senders.model_metadata,
                    &request_id,
                    model_metadata,
                );

                ContinuationDecision::Continue
            }
        }
    }

    async fn handle_undeserializable_message(
        _text: &str,
        deserialization_error: SerdeJsonError,
        _websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> ContinuationDecision {
        ContinuationDecision::Stop(ContinuationStopParameters {
            close_reason: Some(CloseReason {
                code: CloseCode::Invalid,
                description: Some(format!(
                    "Agent message could not be deserialized: {deserialization_error}"
                )),
            }),
        })
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

pub fn ws_agent_socket(cfg: &mut ServiceConfig) {
    cfg.route(
        &ApiPath::agent_socket(AgentIdPathParams::ROUTE_SEGMENT),
        get().to(respond),
    );
}
