use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicU64;

use anyhow::Result;
use async_trait::async_trait;
use log::debug;
use nanoid::nanoid;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_controller_update_result::AgentControllerUpdateResult;
use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::embedding_sender_collection::EmbeddingSenderCollection;
use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::manages_senders::ManagesSenders;
use crate::manages_senders_controller::ManagesSendersController;
use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::sends_rpc_message::SendsRpcMessage;
use crate::sets_desired_state::SetsDesiredState;
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
    pub desired_slots_total: AtomicValue<AtomicI32>,
    pub download_current: AtomicValue<AtomicU64>,
    pub download_filename: RwLock<Option<String>>,
    pub download_indeterminate: AtomicValue<AtomicBool>,
    pub download_total: AtomicValue<AtomicU64>,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub id: String,
    pub issues: RwLock<BTreeSet<AgentIssue>>,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
    pub model_path: RwLock<Option<String>>,
    pub name: Option<String>,
    pub newest_update_version: AtomicValue<AtomicI32>,
    pub slots_processing: AtomicValue<AtomicI32>,
    pub slots_total: AtomicValue<AtomicI32>,
    pub state_application_status_code: AtomicValue<AtomicI32>,
    pub uses_chat_template_override: AtomicValue<AtomicBool>,
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

    pub fn get_download_filename(&self) -> Option<String> {
        self.download_filename.read().clone()
    }

    pub fn get_issues(&self) -> BTreeSet<AgentIssue> {
        self.issues.read().clone()
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

    pub fn get_model_path(&self) -> Option<String> {
        self.model_path.read().clone()
    }

    pub fn set_download_filename(&self, filename: Option<String>) {
        let mut locked_filename = self.download_filename.write();

        *locked_filename = filename;
    }

    pub fn set_issues(&self, issues: BTreeSet<AgentIssue>) {
        let mut locked_issues = self.issues.write();

        *locked_issues = issues;
    }

    pub fn set_model_path(&self, model_path: Option<String>) {
        let mut locked_path = self.model_path.write();

        *locked_path = model_path;
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
        SlotAggregatedStatusSnapshot {
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
            version,
            ..
        }: SlotAggregatedStatusSnapshot,
    ) -> AgentControllerUpdateResult {
        let newest_update_version = self.newest_update_version.get();

        if version < newest_update_version {
            debug!("Discarding update with older version: {version}");

            return AgentControllerUpdateResult::NoMeaningfulChanges;
        }

        let mut changed = false;

        changed |= self.desired_slots_total.set_check(desired_slots_total);
        changed |= self.download_current.set_check(download_current);
        changed |= self
            .download_indeterminate
            .set_check(download_indeterminate);
        changed |= self.download_total.set_check(download_total);
        changed |= self.slots_total.set_check(slots_total);
        changed |= self
            .state_application_status_code
            .set_check(state_application_status as i32);
        changed |= self
            .uses_chat_template_override
            .set_check(uses_chat_template_override);

        self.newest_update_version
            .compare_and_swap(newest_update_version, version);

        if download_filename != self.get_download_filename() {
            changed = true;

            self.set_download_filename(download_filename);
        }

        if issues != self.get_issues() {
            changed = true;

            self.set_issues(issues);
        }

        if model_path != self.get_model_path() {
            changed = true;

            self.set_model_path(model_path);
        }

        if changed {
            AgentControllerUpdateResult::Updated
        } else {
            AgentControllerUpdateResult::NoMeaningfulChanges
        }
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

    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        Ok(AgentControllerSnapshot {
            desired_slots_total: self.desired_slots_total.get(),
            download_current: self.download_current.get(),
            download_filename: self.get_download_filename(),
            download_indeterminate: self.download_indeterminate.get(),
            download_total: self.download_total.get(),
            id: self.id.clone(),
            issues: self.get_issues(),
            model_path: self.get_model_path(),
            name: self.name.clone(),
            slots_processing: self.slots_processing.get(),
            slots_total: self.slots_total.get(),
            state_application_status: self.state_application_status_code.get().try_into()?,
            uses_chat_template_override: self.uses_chat_template_override.get(),
        })
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

#[async_trait]
impl SetsDesiredState for AgentController {
    async fn set_desired_state(&self, desired_state: AgentDesiredState) -> Result<()> {
        self.send_rpc_message(AgentJsonRpcMessage::Notification(
            AgentJsonRpcNotification::SetState(Box::new(SetStateParams { desired_state })),
        ))
        .await
    }
}
