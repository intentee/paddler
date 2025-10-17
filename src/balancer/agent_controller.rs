use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicUsize;

use anyhow::Result;
use async_trait::async_trait;
use log::debug;
use nanoid::nanoid;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use crate::agent::jsonrpc::notification_params::SetStateParams;
use crate::agent::jsonrpc::Message as AgentJsonRpcMessage;
use crate::agent::jsonrpc::Notification as AgentJsonRpcNotification;
use crate::agent::jsonrpc::Request as AgentJsonRpcRequest;
use crate::agent_desired_state::AgentDesiredState;
use crate::agent_issue::AgentIssue;
use crate::atomic_value::AtomicValue;
use crate::balancer::agent_controller_snapshot::AgentControllerSnapshot;
use crate::balancer::agent_controller_update_result::AgentControllerUpdateResult;
use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::balancer::handles_agent_streaming_response::HandlesAgentStreamingResponse;
use crate::balancer::manages_senders::ManagesSenders;
use crate::balancer::manages_senders_controller::ManagesSendersController;
use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::jsonrpc::RequestEnvelope;
use crate::produces_snapshot::ProducesSnapshot;
use crate::request_params::ContinueFromConversationHistoryParams;
use crate::request_params::ContinueFromRawPromptParams;
use crate::request_params::GenerateEmbeddingBatchParams;
use crate::sends_rpc_message::SendsRpcMessage;
use crate::sets_desired_state::SetsDesiredState;
use crate::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

pub struct AgentController {
    pub agent_message_tx: mpsc::UnboundedSender<AgentJsonRpcMessage>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub connection_close_rx: broadcast::Receiver<()>,
    pub desired_slots_total: AtomicValue<AtomicI32>,
    pub download_current: AtomicValue<AtomicUsize>,
    pub download_filename: RwLock<Option<String>>,
    pub download_total: AtomicValue<AtomicUsize>,
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
        self.download_filename
            .read()
            .expect("Poisoned lock on download filename")
            .clone()
    }

    pub fn get_issues(&self) -> BTreeSet<AgentIssue> {
        self.issues.read().expect("Poisoned lock on issues").clone()
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
        self.model_path
            .read()
            .expect("Poisoned lock on model path")
            .clone()
    }

    pub fn set_download_filename(&self, filename: Option<String>) {
        let mut locked_filename = self
            .download_filename
            .write()
            .expect("Poisoned lock on download filename");

        *locked_filename = filename;
    }

    pub fn set_issues(&self, issues: BTreeSet<AgentIssue>) {
        let mut locked_issues = self.issues.write().expect("Poisoned lock on issues");

        *locked_issues = issues;
    }

    pub fn set_model_path(&self, model_path: Option<String>) {
        let mut locked_path = self
            .model_path
            .write()
            .expect("Poisoned lock on model path");

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
            download_total,
            issues,
            model_path,
            slots_processing,
            slots_total,
            state_application_status,
            uses_chat_template_override,
            version,
        }: SlotAggregatedStatusSnapshot,
    ) -> AgentControllerUpdateResult {
        let newest_update_version = self.newest_update_version.get();

        if version < newest_update_version {
            debug!("Discarding update with older version: {version}");

            return AgentControllerUpdateResult::NoMeaningfulChanges;
        }

        let mut changed = false;

        changed = changed || self.desired_slots_total.set_check(desired_slots_total);
        changed = changed || self.download_current.set_check(download_current);
        changed = changed || self.download_total.set_check(download_total);
        changed = changed || self.slots_processing.set_check(slots_processing);
        changed = changed || self.slots_total.set_check(slots_total);
        changed = changed
            || self
                .state_application_status_code
                .set_check(state_application_status as i32);
        changed = changed
            || self
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
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        sender_collection.register_sender(request_id.clone(), response_tx)?;

        self.send_rpc_message(message).await?;

        Ok(ManagesSendersController {
            request_id,
            response_rx,
            response_sender_collection: sender_collection.clone(),
        })
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
            download_filename: self
                .download_filename
                .read()
                .expect("Poisoned lock on download filename")
                .clone(),
            download_total: self.download_total.get(),
            id: self.id.clone(),
            issues: self.get_issues(),
            model_path: self
                .model_path
                .read()
                .expect("Poisoned lock on model path")
                .clone(),
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
            AgentJsonRpcNotification::SetState(SetStateParams { desired_state }),
        ))
        .await
    }
}
