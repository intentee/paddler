use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use log::info;
use log::warn;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::agent::continuous_batch_arbiter::ContinuousBatchArbiter;
use crate::agent::continuous_batch_arbiter_build_outcome::ContinuousBatchArbiterBuildOutcome;
use crate::agent::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;
use crate::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::agent::drain_in_flight_requests::drain_in_flight_requests;
use crate::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::agent::model_metadata_holder::ModelMetadataHolder;
use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
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
    pub continuous_batch_arbiter_handle: Option<ContinuousBatchArbiterHandle>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub slot_aggregated_status_manager: Arc<SlotAggregatedStatusManager>,
}

impl LlamaCppArbiterService {
    async fn apply_state(&mut self, shutdown: &CancellationToken) -> Result<()> {
        self.wait_for_in_flight_requests_to_finish(shutdown).await?;
        self.tear_down_arbiter()?;

        if let Some(applicable_state) = self.agent_applicable_state.clone() {
            self.slot_aggregated_status_manager.reset();

            match ContinuousBatchArbiter::build_from_applicable_state(
                applicable_state,
                self.agent_name.clone(),
                self.desired_slots_total,
                self.model_metadata_holder.clone(),
                self.slot_aggregated_status_manager.clone(),
            ) {
                ContinuousBatchArbiterBuildOutcome::ReadyToSpawn(arbiter) => {
                    self.continuous_batch_arbiter_handle = Some(arbiter.spawn().await?);
                    info!("Reconciled state change applied successfully");
                }
                ContinuousBatchArbiterBuildOutcome::NoModelConfigured => {
                    warn!(
                        "No model configured in applicable state; skipping llama.cpp initialization"
                    );
                }
            }
        }

        self.slot_aggregated_status_manager
            .slot_aggregated_status
            .set_state_application_status(AgentStateApplicationStatus::Applied);

        Ok(())
    }

    async fn wait_for_in_flight_requests_to_finish(
        &self,
        shutdown: &CancellationToken,
    ) -> Result<()> {
        if self.continuous_batch_arbiter_handle.is_some() {
            drain_in_flight_requests(&self.slot_aggregated_status_manager, shutdown).await?;
        }

        Ok(())
    }

    fn tear_down_arbiter(&mut self) -> Result<()> {
        if let Some(arbiter_handle) = self.continuous_batch_arbiter_handle.take() {
            arbiter_handle
                .shutdown()
                .context("Unable to stop arbiter controller")?;
        }

        Ok(())
    }

    fn forward_command(&self, command: ContinuousBatchSchedulerCommand) {
        if let Some(arbiter_handle) = &self.continuous_batch_arbiter_handle {
            if let Err(err) = arbiter_handle.command_tx.send(command) {
                error!("Failed to forward command to scheduler: {err}");
            }
        } else {
            error!("ContinuousBatchArbiterHandle is not initialized");
        }
    }

    async fn try_to_apply_state(&mut self, shutdown: &CancellationToken) {
        if let Err(err) = self.apply_state(shutdown).await {
            error!("Failed to apply reconciled state change: {err}");
        }
    }

    async fn shutdown_arbiter_handle(&mut self) -> Result<()> {
        let Some(handle) = self.continuous_batch_arbiter_handle.take() else {
            return Ok(());
        };

        tokio::task::spawn_blocking(move || handle.shutdown())
            .await
            .context("Arbiter shutdown task panicked")?
            .context("Arbiter shutdown returned an error")
    }
}

#[async_trait]
impl Service for LlamaCppArbiterService {
    fn name(&self) -> &'static str {
        "agent::llamacpp_arbiter_service"
    }

    async fn run(&mut self, shutdown: CancellationToken) -> Result<()> {
        let mut reconciled_state = self.agent_applicable_state_holder.subscribe();
        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let shutdown_outcome = loop {
            tokio::select! {
                biased;
                () = shutdown.cancelled() => break Ok(()),
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

                        self.try_to_apply_state(&shutdown).await;
                    }
                }
                _ = reconciled_state.changed() => {
                    self.agent_applicable_state.clone_from(&reconciled_state.borrow_and_update());
                    self.slot_aggregated_status_manager
                        .slot_aggregated_status
                        .set_state_application_status(AgentStateApplicationStatus::Fresh);

                    self.try_to_apply_state(&shutdown).await;
                }
                Some(request) = self.continue_from_conversation_history_request_rx.recv() => {
                    self.forward_command(
                        ContinuousBatchSchedulerCommand::ContinueFromConversationHistory(request),
                    );
                }
                Some(request) = self.continue_from_raw_prompt_request_rx.recv() => {
                    self.forward_command(
                        ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(request),
                    );
                }
                Some(request) = self.generate_embedding_batch_request_rx.recv() => {
                    self.forward_command(
                        ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(request),
                    );
                }
            }
        };

        if let Err(err) = self.shutdown_arbiter_handle().await {
            error!("Failed to shut down arbiter cleanly: {err:#}");
        }

        shutdown_outcome
    }
}

#[cfg(test)]
mod tests {
    use anyhow::bail;

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn does_not_exit_when_request_channels_close_without_shutdown() -> Result<()> {
        let observation_window = Duration::from_millis(500);
        let shutdown_grace = Duration::from_secs(5);

        let (
            continue_from_conversation_history_request_tx,
            continue_from_conversation_history_request_rx,
        ) = mpsc::unbounded_channel();
        let (continue_from_raw_prompt_request_tx, continue_from_raw_prompt_request_rx) =
            mpsc::unbounded_channel();
        let (generate_embedding_batch_request_tx, generate_embedding_batch_request_rx) =
            mpsc::unbounded_channel();

        let mut service = LlamaCppArbiterService {
            agent_applicable_state: None,
            agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
            agent_name: None,
            continue_from_conversation_history_request_rx,
            continue_from_raw_prompt_request_rx,
            desired_slots_total: 1,
            generate_embedding_batch_request_rx,
            continuous_batch_arbiter_handle: None,
            model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
            slot_aggregated_status_manager: Arc::new(SlotAggregatedStatusManager::new(1)),
        };

        let shutdown = CancellationToken::new();
        let task_token = shutdown.clone();

        let mut join_handle = tokio::spawn(async move { service.run(task_token).await });

        drop(continue_from_conversation_history_request_tx);
        drop(continue_from_raw_prompt_request_tx);
        drop(generate_embedding_batch_request_tx);

        let exited_before_shutdown = tokio::select! {
            join_result = &mut join_handle => Some(join_result),
            () = tokio::time::sleep(observation_window) => None,
        };

        if let Some(join_result) = exited_before_shutdown {
            let inner = join_result.context("service task panicked")?;
            bail!("service exited on channel closure without shutdown: {inner:?}");
        }

        shutdown.cancel();

        tokio::time::timeout(shutdown_grace, join_handle)
            .await
            .context("service did not exit after shutdown")?
            .context("service task panicked")??;

        Ok(())
    }
}
