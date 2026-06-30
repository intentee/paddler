mod agent_socket_controller_context;

use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicU64;

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
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::rpc_message::RpcMessage;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use self::agent_socket_controller_context::AgentSocketControllerContext;
use crate::agent_controller::AgentController;
use crate::agent_controller_pool::AgentControllerPool;
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
use crate::sets_desired_state::SetsDesiredState as _;
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

pub fn register(cfg: &mut ServiceConfig) {
    cfg.service(respond);
}

async fn forward_message_or_log_failure<TResponse>(
    websocket_session_controller: &mut WebSocketSessionController<TResponse>,
    message: TResponse,
) where
    TResponse: RpcMessage + Sync + 'static,
{
    if let Err(err) = websocket_session_controller.send_response(message).await {
        error!("Error sending response: {err}");
    }
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
    ) -> Result<ContinuationDecision> {
        match deserialized_message {
            ManagementJsonRpcMessage::Error(err) => {
                error!("Received error message: {err:?}");

                Ok(ContinuationDecision::Continue)
            }
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::DeregisterAgent,
            ) => {
                connection_close.cancel();

                return Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                    close_reason: None,
                }));
            }
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::RegisterAgent(RegisterAgentParams {
                    name,
                    slot_aggregated_status_snapshot:
                        SlotAggregatedStatusSnapshot {
                            desired_slots_total,
                            download_current,
                            download_filename,
                            download_indeterminate,
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
                    connection_close: connection_close.clone(),
                    desired_slots_total: AtomicValue::<AtomicI32>::new(desired_slots_total),
                    download_current: AtomicValue::<AtomicU64>::new(download_current),
                    download_filename: RwLock::new(download_filename),
                    download_indeterminate: AtomicValue::<AtomicBool>::new(download_indeterminate),
                    download_total: AtomicValue::<AtomicU64>::new(download_total),
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

                let forwarder_close = connection_close.clone();

                rt::spawn(async move {
                    loop {
                        tokio::select! {
                            () = forwarder_close.cancelled() => {
                                break;
                            }
                            result = agent_message_rx.recv() => {
                                if let Some(message) = result {
                                    forward_message_or_log_failure(
                                        &mut websocket_session_controller,
                                        message,
                                    )
                                    .await;
                                } else {
                                    info!("Session channel closed for agent: {}", context.agent_id);
                                    break;
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
                            context.agent_controller_pool.signal_update();
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
                ..
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
                ..
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
                ..
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
                ..
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
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                }),
            ))?)
            .await
        {
            error!("Error sending version: {err:?}");

            return Ok(ContinuationDecision::Stop(ContinuationStopParameters {
                close_reason: None,
            }));
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

    agent_socket_controller.respond(payload, req, app_data.shutdown.clone())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use actix_web::FromRequest as _;
    use actix_web::body::to_bytes;
    use actix_web::http::header;
    use actix_web::test::TestRequest;
    use actix_web::web::Payload;
    use tokio_util::sync::CancellationToken;

    use super::AgentSocketController;
    use super::AgentSocketControllerContext;
    use super::ManagementJsonRpcMessage;
    use super::ManagementJsonRpcNotification;
    use super::RegisterAgentParams;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::continuation_decision::ContinuationDecision;
    use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint as _;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::websocket_session_controller::WebSocketSessionController;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    #[actix_web::test]
    async fn forwarder_task_stops_when_agent_message_channel_closes() {
        log::set_max_level(log::LevelFilter::Trace);

        let agent_id = "agent-forwarder-close".to_owned();
        let agent_controller_pool = Arc::new(AgentControllerPool::default());
        let context = Arc::new(AgentSocketControllerContext {
            agent_controller_pool: agent_controller_pool.clone(),
            agent_id: agent_id.clone(),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
        });

        let (request, mut raw_payload) = TestRequest::get()
            .insert_header((header::CONNECTION, "upgrade"))
            .insert_header((header::UPGRADE, "websocket"))
            .insert_header((header::SEC_WEBSOCKET_VERSION, "13"))
            .insert_header((header::SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_http_parts();
        let payload = Payload::from_request(&request, &mut raw_payload)
            .await
            .unwrap();
        let (response, session, _msg_stream) = actix_ws::handle(&request, payload).unwrap();

        let continuation_decision = AgentSocketController::handle_deserialized_message(
            CancellationToken::new(),
            context,
            ManagementJsonRpcMessage::Notification(ManagementJsonRpcNotification::RegisterAgent(
                RegisterAgentParams {
                    name: None,
                    slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot {
                        desired_slots_total: 0,
                        download_current: 0,
                        download_filename: None,
                        download_indeterminate: false,
                        download_total: 0,
                        issues: BTreeSet::new(),
                        model_path: None,
                        slots_processing: 0,
                        slots_total: 1,
                        state_application_status: AgentStateApplicationStatus::Fresh,
                        uses_chat_template_override: false,
                        version: 0,
                    },
                },
            )),
            WebSocketSessionController::new(session),
        )
        .await
        .unwrap();

        assert_eq!(
            std::mem::discriminant(&continuation_decision),
            std::mem::discriminant(&ContinuationDecision::Continue),
        );

        assert!(
            agent_controller_pool
                .get_agent_controller(&agent_id)
                .is_some()
        );

        assert!(agent_controller_pool.remove_agent_controller(&agent_id));

        let close_frame = to_bytes(response.into_body()).await.unwrap();

        assert!(close_frame.is_empty());
    }

    #[derive(serde::Serialize)]
    struct ForwardableTestMessage;

    impl paddler_messaging::rpc_message::RpcMessage for ForwardableTestMessage {}

    #[actix_web::test]
    async fn forwarding_to_a_disconnected_agent_logs_the_send_failure() {
        log::set_max_level(log::LevelFilter::Trace);

        let (request, mut raw_payload) = TestRequest::default()
            .insert_header(("upgrade", "websocket"))
            .insert_header(("connection", "upgrade"))
            .insert_header(("sec-websocket-version", "13"))
            .insert_header(("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_http_parts();
        let payload = Payload::from_request(&request, &mut raw_payload)
            .await
            .unwrap();

        let (response, session, _message_stream) = actix_ws::handle(&request, payload).unwrap();

        drop(response);

        let mut websocket_session_controller =
            WebSocketSessionController::<ForwardableTestMessage>::new(session);

        super::forward_message_or_log_failure(
            &mut websocket_session_controller,
            ForwardableTestMessage,
        )
        .await;
    }
}
