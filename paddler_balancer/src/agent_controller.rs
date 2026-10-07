use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use nanoid::nanoid;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::model_metadata::ModelMetadata;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_controller_update_result::AgentControllerUpdateResult;
use crate::agent_desired_state_notification::agent_desired_state_notification;
use crate::agent_relay_error::AgentRelayError;
use crate::agent_response_receiver::AgentResponseReceiver;
use crate::agent_response_senders::AgentResponseSenders;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::desired_state_delivery::DesiredStateDelivery;
use crate::response_senders::ResponseSenders;

pub struct AgentController {
    pub agent_message_tx: mpsc::UnboundedSender<AgentJsonRpcMessage>,
    pub agent_response_senders: AgentResponseSenders,
    pub connection_close: CancellationToken,
    pub id: String,
    pub name: Option<String>,
    pub reported_status: RwLock<SlotAggregatedStatusSnapshot>,
    pub slots_processing: AtomicValue<AtomicU64>,
}

impl AgentController {
    pub fn get_chat_template_override(
        &self,
    ) -> Result<AgentResponseReceiver<Option<ChatTemplate>>, AgentRelayError> {
        self.relay_request(
            nanoid!(),
            AgentJsonRpcRequest::GetChatTemplateOverride,
            &self.agent_response_senders.chat_template_override,
        )
    }

    pub fn get_model_metadata(
        &self,
    ) -> Result<AgentResponseReceiver<Option<ModelMetadata>>, AgentRelayError> {
        self.relay_request(
            nanoid!(),
            AgentJsonRpcRequest::GetModelMetadata,
            &self.agent_response_senders.model_metadata,
        )
    }

    pub fn set_desired_state(&self, desired_state: AgentDesiredState) -> DesiredStateDelivery {
        match self
            .agent_message_tx
            .send(agent_desired_state_notification(desired_state))
        {
            Ok(()) => DesiredStateDelivery::Delivered,
            Err(SendError(_undelivered_message)) => DesiredStateDelivery::AgentDisconnected,
        }
    }

    pub fn stop_responding_to(&self, request_id: String) -> Result<(), AgentRelayError> {
        self.send_to_agent(AgentJsonRpcMessage::Notification(
            AgentJsonRpcNotification::StopRespondingTo(request_id),
        ))
    }

    pub fn stream_responses_to<TRequest: AgentStreamingRequest>(
        &self,
        request_id: String,
        request: TRequest,
    ) -> Result<AgentResponseReceiver<TRequest::Response>, AgentRelayError> {
        self.relay_request(
            request_id,
            request.into(),
            TRequest::response_senders(&self.agent_response_senders),
        )
    }

    pub fn update_from_slot_aggregated_status_snapshot(
        &self,
        reported_snapshot: SlotAggregatedStatusSnapshot,
    ) -> AgentControllerUpdateResult {
        let mut current_snapshot = self.reported_status.write();

        if reported_snapshot.version < current_snapshot.version {
            return AgentControllerUpdateResult::NoMeaningfulChanges;
        }

        let status_changed = reported_snapshot.status != current_snapshot.status;

        *current_snapshot = reported_snapshot;

        if status_changed {
            AgentControllerUpdateResult::Updated
        } else {
            AgentControllerUpdateResult::NoMeaningfulChanges
        }
    }

    fn relay_request<TResponse>(
        &self,
        request_id: String,
        request: AgentJsonRpcRequest,
        response_senders: &Arc<ResponseSenders<TResponse>>,
    ) -> Result<AgentResponseReceiver<TResponse>, AgentRelayError> {
        AgentResponseReceiver::register(response_senders, request_id.clone()).and_then(
            |agent_response_receiver| {
                self.send_to_agent(AgentJsonRpcMessage::Request(RequestEnvelope {
                    id: request_id,
                    request,
                }))
                .map(|()| agent_response_receiver)
            },
        )
    }

    fn send_to_agent(&self, message: AgentJsonRpcMessage) -> Result<(), AgentRelayError> {
        self.agent_message_tx
            .send(message)
            .map_err(
                |SendError(_undelivered_message)| AgentRelayError::AgentSocketClosed {
                    agent_id: self.id.clone(),
                },
            )
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
    use std::num::NonZeroU32;
    use std::sync::atomic::AtomicU64;

    use parking_lot::RwLock;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::agent_status::AgentStatus;
    use paddler_messaging::atomic_value::AtomicValue;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::AgentController;
    use crate::agent_controller_update_result::AgentControllerUpdateResult;
    use crate::agent_relay_error::AgentRelayError;
    use crate::agent_response_senders::AgentResponseSenders;

    fn reported(version: u64, slots_total: u64) -> SlotAggregatedStatusSnapshot {
        SlotAggregatedStatusSnapshot {
            status: AgentStatus {
                runtime: AgentRuntimeStatus::Serving {
                    inference_mode: InferenceMode::TextGeneration,
                    slots_total,
                },
                ..AgentStatus::default()
            },
            version,
        }
    }

    fn agent_controller_with(reported_status: SlotAggregatedStatusSnapshot) -> AgentController {
        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();

        AgentController {
            agent_message_tx,
            agent_response_senders: AgentResponseSenders::default(),
            connection_close: CancellationToken::new(),
            id: "agent".to_owned(),
            name: None,
            reported_status: RwLock::new(reported_status),
            slots_processing: AtomicValue::<AtomicU64>::new(0),
        }
    }

    #[test]
    fn keeps_the_newer_status_when_an_older_report_arrives_late() {
        let agent_controller = agent_controller_with(reported(2, 4));

        assert_eq!(
            agent_controller.update_from_slot_aggregated_status_snapshot(reported(1, 8)),
            AgentControllerUpdateResult::NoMeaningfulChanges
        );
        assert_eq!(*agent_controller.reported_status.read(), reported(2, 4));
    }

    #[test]
    fn reports_no_change_for_a_newer_report_with_the_same_status() {
        let agent_controller = agent_controller_with(reported(1, 4));

        assert_eq!(
            agent_controller.update_from_slot_aggregated_status_snapshot(reported(2, 4)),
            AgentControllerUpdateResult::NoMeaningfulChanges
        );
        assert_eq!(agent_controller.reported_status.read().version, 2);
    }

    #[test]
    fn takes_a_newer_report_with_a_changed_status() {
        let agent_controller = agent_controller_with(reported(1, 4));

        assert_eq!(
            agent_controller.update_from_slot_aggregated_status_snapshot(reported(2, 8)),
            AgentControllerUpdateResult::Updated
        );
        assert_eq!(*agent_controller.reported_status.read(), reported(2, 8));
    }

    fn raw_prompt_params() -> ContinueFromRawPromptParams {
        ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: NonZeroU32::MIN,
            raw_prompt: "hello".to_owned(),
        }
    }

    #[test]
    fn fails_to_ask_an_agent_whose_socket_is_gone_for_its_model_metadata() {
        let agent_controller = agent_controller_with(reported(1, 4));

        assert!(matches!(
            agent_controller.get_model_metadata(),
            Err(AgentRelayError::AgentSocketClosed { agent_id }) if agent_id == "agent"
        ));
    }

    #[test]
    fn releases_the_request_id_of_a_stream_to_an_agent_whose_socket_is_gone() {
        let agent_controller = agent_controller_with(reported(1, 4));

        let stream_result =
            agent_controller.stream_responses_to("request".to_owned(), raw_prompt_params());

        assert!(matches!(
            stream_result,
            Err(AgentRelayError::AgentSocketClosed { agent_id }) if agent_id == "agent"
        ));
        assert_eq!(
            agent_controller
                .agent_response_senders
                .generated_tokens
                .with_registered("request", |_response_tx| ()),
            None
        );
    }

    #[test]
    fn refuses_a_second_stream_under_a_request_id_in_flight() {
        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();
        let agent_controller = AgentController {
            agent_message_tx,
            ..agent_controller_with(reported(1, 4))
        };
        let _in_flight_stream = agent_controller
            .stream_responses_to("request".to_owned(), raw_prompt_params())
            .expect("the first stream must reach the agent");

        assert!(matches!(
            agent_controller.stream_responses_to("request".to_owned(), raw_prompt_params()),
            Err(AgentRelayError::RequestIdAlreadyRelayed { request_id }) if request_id == "request"
        ));
    }
}
