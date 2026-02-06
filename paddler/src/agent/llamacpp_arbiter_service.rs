use std::fmt::Debug;
use std::sync::Arc;

use actix::Message;
use actix_web::rt;
use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use log::error;
use log::info;
use log::warn;
use tokio::fs;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::agent::llamacpp_arbiter::LlamaCppArbiter;
use crate::agent::llamacpp_arbiter_handle::LlamaCppArbiterHandle;
use crate::agent::llamacpp_slot::LlamaCppSlot;
use crate::agent::model_metadata_holder::ModelMetadataHolder;
use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::agent_issue::AgentIssue;
use crate::agent_issue_fix::AgentIssueFix;
use crate::agent_state_application_status::AgentStateApplicationStatus;
use crate::service::Service;
use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

pub struct LlamaCppArbiterService {
    pub agent_applicable_state: Option<AgentApplicableState>,
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_name: Option<String>,
    pub continue_from_conversation_history_request_rx:
        mpsc::UnboundedReceiver<ContinueFromConversationHistoryRequest>,
    pub continue_from_raw_prompt_request_rx: mpsc::UnboundedReceiver<ContinueFromRawPromptRequest>,
    pub desired_slots_total: i32,
    pub generate_embedding_batch_request_rx: mpsc::UnboundedReceiver<GenerateEmbeddingBatchRequest>,
    pub llamacpp_arbiter_handle: Option<LlamaCppArbiterHandle>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub slot_aggregated_status_manager: Arc<SlotAggregatedStatusManager>,
}

impl LlamaCppArbiterService {
    async fn apply_state(&mut self) -> Result<()> {
        if let Some(llamacpp_arbiter_handle) = self.llamacpp_arbiter_handle.take() {
            llamacpp_arbiter_handle
                .shutdown()
                .context("Unable to stop arbiter controller")?;
        }

        if let Some(AgentApplicableState {
            chat_template_override,
            inference_parameters,
            model_path,
        }) = self.agent_applicable_state.clone()
        {
            self.slot_aggregated_status_manager.reset();

            if let Some(model_path) = model_path {
                if !fs::try_exists(&model_path).await? {
                    self.slot_aggregated_status_manager
                        .slot_aggregated_status
                        .register_issue(AgentIssue::ModelFileDoesNotExist(
                            model_path.display().to_string(),
                        ));

                    return Err(anyhow!(
                        "Model path does not exist: {}",
                        model_path.display()
                    ));
                }

                let model_path_string = model_path.display().to_string();

                if self
                    .slot_aggregated_status_manager
                    .slot_aggregated_status
                    .has_issue(&AgentIssue::UnableToFindChatTemplate(
                        model_path_string.clone(),
                    ))
                {
                    self.slot_aggregated_status_manager
                        .slot_aggregated_status
                        .set_state_application_status(
                            AgentStateApplicationStatus::AttemptedAndNotAppliable,
                        );

                    return Err(anyhow!(
                        "Unable to establish chat template for model at path: {model_path_string}"
                    ));
                }

                if self
                    .slot_aggregated_status_manager
                    .slot_aggregated_status
                    .has_issue_like(|issue| {
                        matches!(issue, AgentIssue::ChatTemplateDoesNotCompile(_))
                    })
                {
                    self.slot_aggregated_status_manager
                        .slot_aggregated_status
                        .set_state_application_status(
                            AgentStateApplicationStatus::AttemptedAndNotAppliable,
                        );

                    return Err(anyhow!(
                        "Chat template does not compile for model at path: {model_path_string}"
                    ));
                }

                self.slot_aggregated_status_manager
                    .slot_aggregated_status
                    .register_fix(AgentIssueFix::ModelFileExists);
                self.llamacpp_arbiter_handle = Some(
                    LlamaCppArbiter {
                        agent_name: self.agent_name.clone(),
                        chat_template_override,
                        desired_slots_total: self.desired_slots_total,
                        inference_parameters,
                        model_metadata_holder: self.model_metadata_holder.clone(),
                        model_path,
                        model_path_string,
                        slot_aggregated_status_manager: self.slot_aggregated_status_manager.clone(),
                    }
                    .spawn()
                    .await?,
                );
            } else {
                warn!("Model path is not set, skipping llama.cpp initialization");
            }

            info!("Reconciled state change applied successfully");
        }

        self.slot_aggregated_status_manager
            .slot_aggregated_status
            .set_state_application_status(AgentStateApplicationStatus::Applied);

        Ok(())
    }

    async fn forward_request_to_arbiter<TRequest>(
        &mut self,
        request: TRequest,
        mut shutdown: broadcast::Receiver<()>,
    ) where
        TRequest: Message + Debug + Send + 'static,
        TRequest::Result: Send + 'static,
        LlamaCppSlot: actix::Handler<TRequest>,
    {
        if let Some(llamacpp_arbiter_handle) = &self.llamacpp_arbiter_handle {
            let llamacpp_slot_addr = llamacpp_arbiter_handle.llamacpp_slot_addr.clone();

            rt::spawn(async move {
                tokio::select! {
                    _ = shutdown.recv() => {
                        error!("Shutdown received, stopping request processing");
                    }
                    result = llamacpp_slot_addr.send(request) => {
                        if let Err(err) = result {
                            error!("Failed to forward request to arbiter: {err}");
                        }
                    }
                }
            });
        } else {
            error!("LlamaCppArbiterHandle is not initialized");
        }
    }

    async fn try_to_apply_state(&mut self) {
        if let Err(err) = self.apply_state().await {
            error!("Failed to apply reconciled state change: {err}");
        }
    }
}

#[async_trait]
impl Service for LlamaCppArbiterService {
    fn name(&self) -> &'static str {
        "agent::llamacpp_arbiter_service"
    }

    async fn run(&mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        let mut reconciled_state = self.agent_applicable_state_holder.subscribe();
        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = shutdown.recv() => break Ok(()),
                _ = ticker.tick() => {
                    let current_status = self.slot_aggregated_status_manager.slot_aggregated_status.get_state_application_status()?;

                    if current_status.should_try_to_apply() {
                        self.slot_aggregated_status_manager
                            .slot_aggregated_status
                            .set_state_application_status(
                                if matches!(current_status, AgentStateApplicationStatus::AttemptedAndRetrying) {
                                    AgentStateApplicationStatus::Stuck
                                } else {
                                    AgentStateApplicationStatus::AttemptedAndRetrying
                                }
                            );

                        self.try_to_apply_state().await;
                    }
                }
                _ = reconciled_state.changed() => {
                    self.agent_applicable_state = reconciled_state.borrow_and_update().clone();
                    self.slot_aggregated_status_manager
                        .slot_aggregated_status
                        .set_state_application_status(AgentStateApplicationStatus::Fresh);

                    self.try_to_apply_state().await;
                }
                continue_from_conversation_history_request = self.continue_from_conversation_history_request_rx.recv() => {
                    match continue_from_conversation_history_request {
                        Some(continue_from_conversation_history_request) => {
                            self.forward_request_to_arbiter(
                                continue_from_conversation_history_request,
                                shutdown.resubscribe(),
                            ).await
                        }
                        None => {
                            break Err(anyhow!("ContinueFromConversationHistoryRequest channel closed unexpectedly"));
                        }
                    }
                }
                continue_from_raw_prompt_request = self.continue_from_raw_prompt_request_rx.recv() => {
                    match continue_from_raw_prompt_request {
                        Some(continue_from_raw_prompt_request) => {
                            self.forward_request_to_arbiter(
                                continue_from_raw_prompt_request,
                                shutdown.resubscribe(),
                            ).await
                        }
                        None => {
                            break Err(anyhow!("ContinueFromRawPromptRequest channel closed unexpectedly"));
                        }
                    }
                }
                generate_embedding_batch_request = self.generate_embedding_batch_request_rx.recv() => {
                    match generate_embedding_batch_request {
                        Some(generate_embedding_batch_request) => {
                            self.forward_request_to_arbiter(
                                generate_embedding_batch_request,
                                shutdown.resubscribe(),
                            ).await
                        }
                        None => {
                            break Err(anyhow!("GenerateEmbeddingBatchRequest channel closed unexpectedly"));
                        }
                    }
                }
            }
        }
    }
}
