use std::num::NonZeroU16;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use parking_lot::Mutex;
use parking_lot::RwLock;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::model_metadata::ModelMetadata;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_outbox::outbox::Outbox;

use crate::accept_newer_status_report::accept_newer_status_report;
use crate::agent_controller_params::AgentControllerParams;
use crate::agent_controller_update_result::AgentControllerUpdateResult;
use crate::agent_desired_state_notification::agent_desired_state_notification;
use crate::agent_response_receiver::AgentResponseReceiver;
use crate::agent_response_senders::AgentResponseSenders;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::balancer_applicable_state::BalancerApplicableState;
use crate::response_senders::ResponseSenders;

pub struct AgentController {
    pub agent_outbox: Outbox<AgentJsonRpcMessage>,
    pub agent_response_senders: AgentResponseSenders,
    pub connection_close: CancellationToken,
    pub id: String,
    pub name: Option<String>,
    pub next_deliverable_desired_state_version: Mutex<u64>,
    pub reported_status: RwLock<SlotAggregatedStatusSnapshot>,
    pub slots_processing: AtomicValue<AtomicU64>,
}

impl AgentController {
    #[must_use]
    pub fn new(
        AgentControllerParams {
            connection_close,
            id,
            name,
            reported_status,
        }: AgentControllerParams,
    ) -> Self {
        Self {
            agent_outbox: Outbox::default(),
            agent_response_senders: AgentResponseSenders::default(),
            connection_close,
            id,
            name,
            next_deliverable_desired_state_version: Mutex::new(0),
            reported_status: RwLock::new(reported_status),
            slots_processing: AtomicValue::<AtomicU64>::new(0),
        }
    }

    #[must_use]
    pub fn get_chat_template_override(&self) -> AgentResponseReceiver<Option<ChatTemplate>> {
        self.relay_request(
            AgentJsonRpcRequest::GetChatTemplateOverride,
            &self.agent_response_senders.chat_template_override,
        )
    }

    #[must_use]
    pub fn get_model_metadata(&self) -> AgentResponseReceiver<Option<ModelMetadata>> {
        self.relay_request(
            AgentJsonRpcRequest::GetModelMetadata,
            &self.agent_response_senders.model_metadata,
        )
    }

    pub fn set_desired_state(
        &self,
        BalancerApplicableState {
            agent_desired_state,
            version,
            ..
        }: &BalancerApplicableState,
    ) {
        let mut next_deliverable_desired_state_version =
            self.next_deliverable_desired_state_version.lock();

        if *version < *next_deliverable_desired_state_version {
            return;
        }

        *next_deliverable_desired_state_version = version + 1;
        self.agent_outbox.push(agent_desired_state_notification(
            agent_desired_state.clone(),
        ));
    }

    pub fn stop_responding_to(&self, request_id: String) {
        self.agent_outbox.push(AgentJsonRpcMessage::Notification(
            AgentJsonRpcNotification::StopRespondingTo(request_id),
        ));
    }

    #[must_use]
    pub fn stream_responses_to<TRequest: AgentStreamingRequest>(
        &self,
        request: TRequest,
        reserved_slots: NonZeroU16,
    ) -> AgentResponseReceiver<TRequest::Response> {
        self.relay_request(
            request.into_reserved_agent_request(reserved_slots),
            TRequest::response_senders(&self.agent_response_senders),
        )
    }

    pub fn update_from_slot_aggregated_status_snapshot(
        &self,
        reported_snapshot: SlotAggregatedStatusSnapshot,
    ) -> AgentControllerUpdateResult {
        accept_newer_status_report(&mut self.reported_status.write(), reported_snapshot)
    }

    fn relay_request<TResponse>(
        &self,
        request: AgentJsonRpcRequest,
        response_senders: &Arc<ResponseSenders<TResponse>>,
    ) -> AgentResponseReceiver<TResponse> {
        let agent_response_receiver = AgentResponseReceiver::register(response_senders);

        self.agent_outbox
            .push(AgentJsonRpcMessage::Request(RequestEnvelope {
                id: agent_response_receiver
                    .request_registration
                    .request_id
                    .clone(),
                request,
            }));

        agent_response_receiver
    }
}

impl ProducesSnapshot for AgentController {
    type Snapshot = AgentControllerSnapshot;

    fn make_snapshot(&self) -> Self::Snapshot {
        AgentControllerSnapshot {
            id: self.id.clone(),
            name: self.name.clone(),
            slots_processing: self.slots_processing.get(),
            status: self.reported_status.read().status.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::FutureExt as _;
    use serde_json::Value;
    use serde_json::to_value;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::AgentController;
    use crate::agent_controller_params::AgentControllerParams;
    use crate::agent_desired_state_notification::agent_desired_state_notification;
    use crate::balancer_applicable_state::BalancerApplicableState;

    fn desired_state_with_model(model_uri: &str) -> BalancerDesiredState {
        BalancerDesiredState {
            model: AgentDesiredModel::Uri(model_uri.to_owned()),
            ..BalancerDesiredState::default()
        }
    }

    #[test]
    fn a_desired_state_arriving_after_a_newer_one_is_not_sent_to_the_agent() {
        let replaced_state =
            BalancerApplicableState::initial(desired_state_with_model("agent:///replaced.gguf"));
        let newest_state =
            replaced_state.superseded_by(desired_state_with_model("agent:///newest.gguf"));
        let agent_controller = AgentController::new(AgentControllerParams {
            connection_close: CancellationToken::new(),
            id: "agent".to_owned(),
            name: None,
            reported_status: SlotAggregatedStatusSnapshot::default(),
        });

        agent_controller.set_desired_state(&newest_state);
        agent_controller.set_desired_state(&replaced_state);

        let mut sent_messages = Vec::<Value>::new();

        while let Some(message) = agent_controller.agent_outbox.next().now_or_never() {
            sent_messages.push(to_value(message).unwrap());
        }

        assert_eq!(
            sent_messages,
            vec![
                to_value(agent_desired_state_notification(
                    newest_state.agent_desired_state
                ))
                .unwrap()
            ]
        );
    }
}
