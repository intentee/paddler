use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use log::info;
use log::warn;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use crate::continuous_batch_arbiter::ContinuousBatchArbiter;
use crate::continuous_batch_arbiter_build_outcome::ContinuousBatchArbiterBuildOutcome;
use crate::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;
use crate::continuous_batch_arbiter_spawn_outcome::ContinuousBatchArbiterSpawnOutcome;
use crate::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use crate::drain_in_flight_requests::drain_in_flight_requests;
use crate::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use crate::model_metadata_holder::ModelMetadataHolder;
use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

async fn apply_state(
    shutdown: &CancellationToken,
    agent_applicable_state: Option<&AgentApplicableState>,
    agent_name: Option<&str>,
    desired_slots_total: i32,
    model_metadata_holder: &Arc<ModelMetadataHolder>,
    slot_aggregated_status_manager: &Arc<SlotAggregatedStatusManager>,
    continuous_batch_arbiter_handle: &mut Option<ContinuousBatchArbiterHandle>,
) -> Result<()> {
    wait_for_in_flight_requests_to_finish(
        shutdown,
        continuous_batch_arbiter_handle.as_ref(),
        slot_aggregated_status_manager,
    )
    .await?;
    shutdown_arbiter_handle(continuous_batch_arbiter_handle).await?;

    if let Some(applicable_state) = agent_applicable_state.cloned() {
        slot_aggregated_status_manager.reset();

        match ContinuousBatchArbiter::build_from_applicable_state(
            applicable_state,
            agent_name.map(str::to_owned),
            desired_slots_total,
            model_metadata_holder.clone(),
            slot_aggregated_status_manager.clone(),
        ) {
            ContinuousBatchArbiterBuildOutcome::ReadyToSpawn(arbiter) => {
                match arbiter.spawn(shutdown).await? {
                    ContinuousBatchArbiterSpawnOutcome::Ready(handle) => {
                        *continuous_batch_arbiter_handle = Some(handle);
                        info!("Reconciled state change applied successfully");
                    }
                    ContinuousBatchArbiterSpawnOutcome::Cancelled => {
                        info!("Model load was cancelled by shutdown before it finished");
                    }
                }
            }
            ContinuousBatchArbiterBuildOutcome::NoModelConfigured => {
                warn!("No model configured in applicable state; skipping llama.cpp initialization");
            }
        }
    }

    slot_aggregated_status_manager
        .slot_aggregated_status
        .set_state_application_status(AgentStateApplicationStatus::Applied);

    Ok(())
}

fn forward_command(
    continuous_batch_arbiter_handle: Option<&ContinuousBatchArbiterHandle>,
    command: ContinuousBatchSchedulerCommand,
) {
    if let Some(arbiter_handle) = continuous_batch_arbiter_handle {
        if let Err(err) = arbiter_handle.command_tx.send(command) {
            error!("Failed to forward command to scheduler: {err}");
        }
    } else {
        error!("ContinuousBatchArbiterHandle is not initialized");
    }
}

async fn shutdown_arbiter_handle(
    continuous_batch_arbiter_handle: &mut Option<ContinuousBatchArbiterHandle>,
) -> Result<()> {
    let Some(handle) = continuous_batch_arbiter_handle.take() else {
        return Ok(());
    };

    tokio::task::spawn_blocking(move || handle.shutdown())
        .await
        .context("Arbiter shutdown task panicked")?
        .context("Arbiter shutdown returned an error")
}

async fn try_to_apply_state(
    shutdown: &CancellationToken,
    agent_applicable_state: Option<&AgentApplicableState>,
    agent_name: Option<&str>,
    desired_slots_total: i32,
    model_metadata_holder: &Arc<ModelMetadataHolder>,
    slot_aggregated_status_manager: &Arc<SlotAggregatedStatusManager>,
    continuous_batch_arbiter_handle: &mut Option<ContinuousBatchArbiterHandle>,
) {
    if let Err(err) = apply_state(
        shutdown,
        agent_applicable_state,
        agent_name,
        desired_slots_total,
        model_metadata_holder,
        slot_aggregated_status_manager,
        continuous_batch_arbiter_handle,
    )
    .await
    {
        error!("Failed to apply reconciled state change: {err}");
    }
}

async fn wait_for_in_flight_requests_to_finish(
    shutdown: &CancellationToken,
    continuous_batch_arbiter_handle: Option<&ContinuousBatchArbiterHandle>,
    slot_aggregated_status_manager: &Arc<SlotAggregatedStatusManager>,
) -> Result<()> {
    if continuous_batch_arbiter_handle.is_some() {
        drain_in_flight_requests(slot_aggregated_status_manager, shutdown).await?;
    }

    Ok(())
}

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

#[async_trait]
impl Service for LlamaCppArbiterService {
    fn name(&self) -> &'static str {
        "agent::llamacpp_arbiter_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let Self {
            mut agent_applicable_state,
            agent_applicable_state_holder,
            agent_name,
            mut continue_from_conversation_history_request_rx,
            mut continue_from_raw_prompt_request_rx,
            desired_slots_total,
            mut generate_embedding_batch_request_rx,
            mut continuous_batch_arbiter_handle,
            model_metadata_holder,
            slot_aggregated_status_manager,
        } = *self;

        let mut reconciled_state = agent_applicable_state_holder.subscribe();
        let mut ticker = interval(Duration::from_secs(1));

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let shutdown_outcome = loop {
            tokio::select! {
                biased;
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    let current_status = slot_aggregated_status_manager.slot_aggregated_status.get_state_application_status()?;

                    if current_status.should_try_to_apply() {
                        slot_aggregated_status_manager
                            .slot_aggregated_status
                            .set_state_application_status(
                                if matches!(current_status, AgentStateApplicationStatus::AttemptedAndRetrying) {
                                    AgentStateApplicationStatus::Stuck
                                } else {
                                    AgentStateApplicationStatus::AttemptedAndRetrying
                                }
                            );

                        try_to_apply_state(
                            &shutdown,
                            agent_applicable_state.as_ref(),
                            agent_name.as_deref(),
                            desired_slots_total,
                            &model_metadata_holder,
                            &slot_aggregated_status_manager,
                            &mut continuous_batch_arbiter_handle,
                        ).await;
                    }
                }
                _ = reconciled_state.changed() => {
                    agent_applicable_state.clone_from(&reconciled_state.borrow_and_update());
                    slot_aggregated_status_manager
                        .slot_aggregated_status
                        .set_state_application_status(AgentStateApplicationStatus::Fresh);

                    try_to_apply_state(
                        &shutdown,
                        agent_applicable_state.as_ref(),
                        agent_name.as_deref(),
                        desired_slots_total,
                        &model_metadata_holder,
                        &slot_aggregated_status_manager,
                        &mut continuous_batch_arbiter_handle,
                    ).await;
                }
                Some(request) = continue_from_conversation_history_request_rx.recv() => {
                    forward_command(
                        continuous_batch_arbiter_handle.as_ref(),
                        ContinuousBatchSchedulerCommand::ContinueFromConversationHistory(request),
                    );
                }
                Some(request) = continue_from_raw_prompt_request_rx.recv() => {
                    forward_command(
                        continuous_batch_arbiter_handle.as_ref(),
                        ContinuousBatchSchedulerCommand::ContinueFromRawPrompt(request),
                    );
                }
                Some(request) = generate_embedding_batch_request_rx.recv() => {
                    forward_command(
                        continuous_batch_arbiter_handle.as_ref(),
                        ContinuousBatchSchedulerCommand::GenerateEmbeddingBatch(request),
                    );
                }
            }
        };

        if let Err(err) = shutdown_arbiter_handle(&mut continuous_batch_arbiter_handle).await {
            error!("Failed to shut down arbiter cleanly: {err:#}");
        }

        shutdown_outcome
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::sync::mpsc::channel as std_channel;
    use std::thread;

    use anyhow::bail;

    use super::*;

    fn spawn_arbiter_handle_with_live_receiver() -> (
        ContinuousBatchArbiterHandle,
        std::sync::mpsc::Receiver<ContinuousBatchSchedulerCommand>,
    ) {
        let (command_tx, command_rx) = std_channel();
        let scheduler_thread_handle = thread::spawn(|| Ok(()));

        (
            ContinuousBatchArbiterHandle {
                command_tx,
                scheduler_thread_handle,
            },
            command_rx,
        )
    }

    #[test]
    fn forward_command_delivers_command_when_handle_present() {
        let (arbiter_handle, command_rx) = spawn_arbiter_handle_with_live_receiver();

        forward_command(
            Some(&arbiter_handle),
            ContinuousBatchSchedulerCommand::Shutdown,
        );

        let delivered = command_rx.recv().unwrap();

        assert_eq!(
            discriminant(&delivered),
            discriminant(&ContinuousBatchSchedulerCommand::Shutdown),
        );
    }

    #[test]
    fn forward_command_logs_error_when_receiver_dropped() {
        let (arbiter_handle, command_rx) = spawn_arbiter_handle_with_live_receiver();

        drop(command_rx);

        forward_command(
            Some(&arbiter_handle),
            ContinuousBatchSchedulerCommand::Shutdown,
        );
    }

    #[test]
    fn forward_command_logs_error_when_handle_absent() {
        forward_command(None, ContinuousBatchSchedulerCommand::Shutdown);
    }

    #[tokio::test]
    async fn wait_for_in_flight_requests_drains_when_handle_present() {
        let (arbiter_handle, _command_rx) = spawn_arbiter_handle_with_live_receiver();
        let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(1));
        let shutdown = CancellationToken::new();

        wait_for_in_flight_requests_to_finish(
            &shutdown,
            Some(&arbiter_handle),
            &slot_aggregated_status_manager,
        )
        .await
        .unwrap();

        arbiter_handle.shutdown().unwrap();
    }

    #[tokio::test]
    async fn wait_for_in_flight_requests_returns_immediately_without_handle() {
        let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(1));
        let shutdown = CancellationToken::new();

        wait_for_in_flight_requests_to_finish(&shutdown, None, &slot_aggregated_status_manager)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn apply_state_without_model_marks_status_applied() {
        let model_metadata_holder = Arc::new(ModelMetadataHolder::default());
        let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(1));
        let shutdown = CancellationToken::new();
        let mut continuous_batch_arbiter_handle: Option<ContinuousBatchArbiterHandle> = None;

        apply_state(
            &shutdown,
            None,
            None,
            1,
            &model_metadata_holder,
            &slot_aggregated_status_manager,
            &mut continuous_batch_arbiter_handle,
        )
        .await
        .unwrap();

        assert_eq!(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .get_state_application_status()
                .unwrap(),
            AgentStateApplicationStatus::Applied,
        );
    }

    #[tokio::test]
    async fn shutdown_arbiter_handle_returns_ok_when_handle_absent() {
        let mut continuous_batch_arbiter_handle: Option<ContinuousBatchArbiterHandle> = None;

        shutdown_arbiter_handle(&mut continuous_batch_arbiter_handle)
            .await
            .unwrap();

        assert!(continuous_batch_arbiter_handle.is_none());
    }

    #[tokio::test]
    async fn shutdown_arbiter_handle_joins_and_clears_present_handle() {
        let (arbiter_handle, command_rx) = spawn_arbiter_handle_with_live_receiver();
        let mut continuous_batch_arbiter_handle = Some(arbiter_handle);

        shutdown_arbiter_handle(&mut continuous_batch_arbiter_handle)
            .await
            .unwrap();

        assert!(continuous_batch_arbiter_handle.is_none());

        let delivered = command_rx.recv().unwrap();

        assert_eq!(
            discriminant(&delivered),
            discriminant(&ContinuousBatchSchedulerCommand::Shutdown),
        );
    }

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

        let service = LlamaCppArbiterService {
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

        let mut join_handle = tokio::spawn(async move { Box::new(service).run(task_token).await });

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
