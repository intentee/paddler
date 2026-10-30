use std::sync::Arc;

use actix_web::rt;
use log::error;
use log::info;
use parking_lot::RwLock;
use tokio::select;
use tokio_util::sync::CancellationToken;

use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_messaging::management_socket::balancer::notification::Notification as ManagementJsonRpcNotification;
use paddler_messaging::management_socket::balancer::notification_params::register_agent_params::RegisterAgentParams;
use paddler_messaging::management_socket::balancer::notification_params::update_agent_status_params::UpdateAgentStatusParams;

use super::agent_socket_registration::AgentSocketRegistration;
use crate::agent_controller::AgentController;
use crate::agent_controller_params::AgentControllerParams;
use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_controller_registration::AgentControllerRegistration;
use crate::agent_controller_update_result::AgentControllerUpdateResult;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::continuation_decision::ContinuationDecision;
use crate::controls_session::ControlsSession as _;
use crate::registered_agent_controller_guard::RegisteredAgentControllerGuard;
use crate::websocket_close_cause::WebSocketCloseCause;
use crate::websocket_session_controller::WebSocketSessionController;

pub struct AgentSocketControllerContext {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_id: String,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub registration: RwLock<AgentSocketRegistration>,
}

impl AgentSocketControllerContext {
    pub fn handle_message(
        &self,
        connection_close: CancellationToken,
        message: ManagementJsonRpcMessage,
        websocket_session_controller: WebSocketSessionController<AgentJsonRpcMessage>,
    ) -> ContinuationDecision {
        match message {
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::DeregisterAgent,
            ) => {
                connection_close.cancel();

                ContinuationDecision::Stop(WebSocketCloseCause::AgentDeregistered)
            }
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::RegisterAgent(register_agent_params),
            ) => self.register_agent(
                connection_close,
                register_agent_params,
                websocket_session_controller,
            ),
            ManagementJsonRpcMessage::Notification(
                ManagementJsonRpcNotification::UpdateAgentStatus(update_agent_status_params),
            ) => self.update_agent_status(update_agent_status_params),
            ManagementJsonRpcMessage::Response(ResponseEnvelope {
                request_id,
                response,
                ..
            }) => self.forward_response(&request_id, response),
        }
    }

    fn forward_response(
        &self,
        request_id: &str,
        response: AgentJsonRpcResponse,
    ) -> ContinuationDecision {
        match &*self.registration.read() {
            AgentSocketRegistration::AwaitingRegistration => {
                error!(
                    "Rejecting a response from agent {} before its registration",
                    self.agent_id
                );

                ContinuationDecision::Stop(WebSocketCloseCause::AgentResponseBeforeRegistration)
            }
            AgentSocketRegistration::Registered(agent_controller) => {
                agent_controller
                    .agent_response_senders
                    .forward(request_id, response);

                ContinuationDecision::Continue
            }
        }
    }

    fn register_agent(
        &self,
        connection_close: CancellationToken,
        RegisterAgentParams {
            name,
            slot_aggregated_status_snapshot,
        }: RegisterAgentParams,
        mut websocket_session_controller: WebSocketSessionController<AgentJsonRpcMessage>,
    ) -> ContinuationDecision {
        let agent_controller = Arc::new(AgentController::new(AgentControllerParams {
            connection_close: connection_close.clone(),
            id: self.agent_id.clone(),
            name,
            reported_status: slot_aggregated_status_snapshot,
        }));

        let registered_agent_controller_guard =
            match self.agent_controller_pool.register_agent_controller(
                agent_controller.clone(),
                &self.balancer_applicable_state_holder,
            ) {
                AgentControllerRegistration::DuplicateAgentId => {
                    error!("Rejecting a second registration of agent {}", self.agent_id);

                    return ContinuationDecision::Stop(WebSocketCloseCause::AgentAlreadyRegistered);
                }
                AgentControllerRegistration::Registered(registered_agent_controller_guard) => {
                    registered_agent_controller_guard
                }
            };

        *self.registration.write() = AgentSocketRegistration::Registered(agent_controller);

        info!("Registered agent: {}", self.agent_id);

        rt::spawn(async move {
            let RegisteredAgentControllerGuard {
                agent_controller, ..
            } = &registered_agent_controller_guard;

            loop {
                select! {
                    () = connection_close.cancelled() => {
                        break;
                    }
                    message = agent_controller.agent_outbox.next() => {
                        websocket_session_controller.send_response_safe(message).await;
                    }
                }
            }
        });

        ContinuationDecision::Continue
    }

    fn update_agent_status(
        &self,
        UpdateAgentStatusParams {
            slot_aggregated_status_snapshot,
        }: UpdateAgentStatusParams,
    ) -> ContinuationDecision {
        match &*self.registration.read() {
            AgentSocketRegistration::AwaitingRegistration => {
                error!(
                    "Rejecting a status update from agent {} before its registration",
                    self.agent_id
                );

                ContinuationDecision::Stop(WebSocketCloseCause::AgentStatusBeforeRegistration)
            }
            AgentSocketRegistration::Registered(agent_controller) => {
                match agent_controller
                    .update_from_slot_aggregated_status_snapshot(slot_aggregated_status_snapshot)
                {
                    AgentControllerUpdateResult::NoMeaningfulChanges => {}
                    AgentControllerUpdateResult::Updated => {
                        self.agent_controller_pool.signal_update();
                    }
                }

                ContinuationDecision::Continue
            }
        }
    }
}
