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
use crate::continuous_batch_arbiter::ContinuousBatchArbiter;
use crate::continuous_batch_arbiter_build_outcome::ContinuousBatchArbiterBuildOutcome;
use crate::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;
use crate::continuous_batch_arbiter_spawn_outcome::ContinuousBatchArbiterSpawnOutcome;
use crate::continuous_batch_preparation_request::ContinuousBatchPreparationRequest;
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
    slot_aggregated_status_manager.reset();
    shutdown_arbiter_handle(continuous_batch_arbiter_handle).await?;

    if let Some(applicable_state) = agent_applicable_state.cloned() {
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

fn forward_request(
    continuous_batch_arbiter_handle: Option<&ContinuousBatchArbiterHandle>,
    request: ContinuousBatchPreparationRequest,
) {
    if let Some(arbiter_handle) = continuous_batch_arbiter_handle {
        arbiter_handle.request_preparer.prepare(request);
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

    handle
        .shutdown()
        .await
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

pub struct LlamaCppArbiterService {
    pub agent_applicable_state: Option<AgentApplicableState>,
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_name: Option<String>,
    pub continuous_batch_arbiter_handle: Option<ContinuousBatchArbiterHandle>,
    pub continuous_batch_preparation_request_rx:
        mpsc::UnboundedReceiver<ContinuousBatchPreparationRequest>,
    pub desired_slots_total: i32,
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
            mut continuous_batch_arbiter_handle,
            mut continuous_batch_preparation_request_rx,
            desired_slots_total,
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
                    let current_status = slot_aggregated_status_manager.slot_aggregated_status.get_state_application_status();

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
                request = continuous_batch_preparation_request_rx.recv() => match request {
                    Some(request) => forward_request(continuous_batch_arbiter_handle.as_ref(), request),
                    None => break Ok(()),
                },
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
    use std::sync::Arc;

    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service;

    use super::LlamaCppArbiterService;
    use super::apply_state;
    use super::forward_request;
    use super::shutdown_arbiter_handle;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
    use crate::continuous_batch_arbiter_handle::ContinuousBatchArbiterHandle;
    use crate::continuous_batch_preparation_request::ContinuousBatchPreparationRequest;
    use crate::from_request_params::FromRequestParams as _;
    use crate::model_metadata_holder::ModelMetadataHolder;
    use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

    #[tokio::test]
    async fn forward_request_releases_the_request_when_no_arbiter_is_running() {
        let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(1));
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

        forward_request(
            None,
            ContinuousBatchPreparationRequest::ContinueFromRawPrompt(
                ContinueFromRawPromptRequest::from_request_params(
                    ContinueFromRawPromptParams {
                        grammar: None,
                        max_tokens: 1,
                        raw_prompt: "Hello".to_owned(),
                    },
                    generated_tokens_tx,
                    generate_tokens_stop_rx,
                    slot_aggregated_status_manager
                        .slot_aggregated_status
                        .clone(),
                ),
            ),
        );

        assert_eq!(generated_tokens_rx.recv().await, None);
        assert_eq!(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .slots_processing_count(),
            0
        );
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
                .get_state_application_status(),
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

    #[tokio::test(flavor = "multi_thread")]
    async fn exits_when_its_request_channel_closes() {
        let (continuous_batch_preparation_request_tx, continuous_batch_preparation_request_rx) =
            mpsc::unbounded_channel();
        let service = LlamaCppArbiterService {
            agent_applicable_state: None,
            agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
            agent_name: None,
            continuous_batch_arbiter_handle: None,
            continuous_batch_preparation_request_rx,
            desired_slots_total: 1,
            model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
            slot_aggregated_status_manager: Arc::new(SlotAggregatedStatusManager::new(1)),
        };

        drop(continuous_batch_preparation_request_tx);

        Box::new(service)
            .run(CancellationToken::new())
            .await
            .expect("the service must finish cleanly once no request can arrive");
    }
}
