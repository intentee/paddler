use std::sync::Arc;
use std::sync::atomic::AtomicI32;

use anyhow::Result;
use async_trait::async_trait;
use nanoid::nanoid;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_controller_update_result::AgentControllerUpdateResult;
use crate::agent_status::AgentStatus;
use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::desired_state_delivery::DesiredStateDelivery;
use crate::embedding_sender_collection::EmbeddingSenderCollection;
use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::manages_senders::ManagesSenders;
use crate::manages_senders_controller::ManagesSendersController;
use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::sends_rpc_message::SendsRpcMessage;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::notification_params::set_state_params::SetStateParams;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::produces_snapshot::ProducesSnapshot;

pub struct AgentController {
    pub agent_message_tx: mpsc::UnboundedSender<AgentJsonRpcMessage>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub connection_close: CancellationToken,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub id: String,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
    pub name: Option<String>,
    pub slots_processing: AtomicValue<AtomicI32>,
    pub status: RwLock<AgentStatus>,
}

impl AgentController {
    pub async fn get_chat_template_override(
        &self,
    ) -> Result<ManagesSendersController<ChatTemplateOverrideSenderCollection>> {
        self.get_oneshot_response(
            AgentJsonRpcRequest::GetChatTemplateOverride,
            self.chat_template_override_sender_collection.clone(),
        )
        .await
    }

    pub async fn get_model_metadata(
        &self,
    ) -> Result<ManagesSendersController<ModelMetadataSenderCollection>> {
        self.get_oneshot_response(
            AgentJsonRpcRequest::GetModelMetadata,
            self.model_metadata_sender_collection.clone(),
        )
        .await
    }

    pub fn set_desired_state(&self, desired_state: AgentDesiredState) -> DesiredStateDelivery {
        match self
            .agent_message_tx
            .send(AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::SetState(Box::new(SetStateParams { desired_state })),
            )) {
            Ok(()) => DesiredStateDelivery::Delivered,
            Err(SendError(_undelivered_message)) => DesiredStateDelivery::AgentDisconnected,
        }
    }

    pub async fn stop_responding_to(&self, request_id: String) -> Result<()> {
        self.send_rpc_message(AgentJsonRpcMessage::Notification(
            AgentJsonRpcNotification::StopRespondingTo(request_id),
        ))
        .await?;

        Ok(())
    }

    pub fn update_from_slot_aggregated_status_snapshot(
        &self,
        slot_aggregated_status_snapshot: SlotAggregatedStatusSnapshot,
    ) -> AgentControllerUpdateResult {
        self.status
            .write()
            .absorb(AgentStatus::from(slot_aggregated_status_snapshot))
    }

    async fn get_oneshot_response<TManagesSenders: ManagesSenders>(
        &self,
        request: AgentJsonRpcRequest,
        sender_collection: Arc<TManagesSenders>,
    ) -> Result<ManagesSendersController<TManagesSenders>> {
        let request_id: String = nanoid!();

        self.send_rpc_message(AgentJsonRpcMessage::Request(RequestEnvelope {
            id: request_id.clone(),
            request,
        }))
        .await?;

        ManagesSendersController::from_request_id(request_id, sender_collection)
    }

    async fn receiver_from_message<TManagesSenders: ManagesSenders>(
        &self,
        request_id: String,
        sender_collection: Arc<TManagesSenders>,
        message: AgentJsonRpcMessage,
    ) -> Result<ManagesSendersController<TManagesSenders>> {
        let receive_response_controller =
            ManagesSendersController::from_request_id(request_id, sender_collection)?;

        self.send_rpc_message(message).await?;

        Ok(receive_response_controller)
    }
}

#[async_trait]
impl HandlesAgentStreamingResponse<ContinueFromConversationHistoryParams<ValidatedParametersSchema>>
    for AgentController
{
    type SenderCollection = GenerateTokensSenderCollection;

    async fn handle_streaming_response(
        &self,
        request_id: String,
        params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<ManagesSendersController<Self::SenderCollection>> {
        self.receiver_from_message(
            request_id.clone(),
            self.generate_tokens_sender_collection.clone(),
            AgentJsonRpcMessage::Request(RequestEnvelope {
                id: request_id,
                request: params.into(),
            }),
        )
        .await
    }
}

#[async_trait]
impl HandlesAgentStreamingResponse<ContinueFromRawPromptParams> for AgentController {
    type SenderCollection = GenerateTokensSenderCollection;

    async fn handle_streaming_response(
        &self,
        request_id: String,
        params: ContinueFromRawPromptParams,
    ) -> Result<ManagesSendersController<Self::SenderCollection>> {
        self.receiver_from_message(
            request_id.clone(),
            self.generate_tokens_sender_collection.clone(),
            AgentJsonRpcMessage::Request(RequestEnvelope {
                id: request_id,
                request: params.into(),
            }),
        )
        .await
    }
}

#[async_trait]
impl HandlesAgentStreamingResponse<GenerateEmbeddingBatchParams> for AgentController {
    type SenderCollection = EmbeddingSenderCollection;

    async fn handle_streaming_response(
        &self,
        request_id: String,
        params: GenerateEmbeddingBatchParams,
    ) -> Result<ManagesSendersController<Self::SenderCollection>> {
        self.receiver_from_message(
            request_id.clone(),
            self.embedding_sender_collection.clone(),
            AgentJsonRpcMessage::Request(RequestEnvelope {
                id: request_id,
                request: params.into(),
            }),
        )
        .await
    }
}

impl ProducesSnapshot for AgentController {
    type Snapshot = AgentControllerSnapshot;

    fn make_snapshot(&self) -> Self::Snapshot {
        let AgentStatus {
            desired_slots_total,
            download_current,
            download_filename,
            download_indeterminate,
            download_total,
            issues,
            model_path,
            slots_total,
            state_application_status,
            uses_chat_template_override,
            ..
        } = self.status.read().clone();

        AgentControllerSnapshot {
            desired_slots_total,
            download_current,
            download_filename,
            download_indeterminate,
            download_total,
            id: self.id.clone(),
            issues,
            model_path,
            name: self.name.clone(),
            slots_processing: self.slots_processing.get(),
            slots_total,
            state_application_status,
            uses_chat_template_override,
        }
    }
}

#[async_trait]
impl SendsRpcMessage for AgentController {
    type Message = AgentJsonRpcMessage;

    async fn send_rpc_message(&self, message: Self::Message) -> Result<()> {
        self.agent_message_tx.send(message)?;

        Ok(())
    }
}
