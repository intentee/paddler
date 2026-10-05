#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_agent::continuous_batch_arbiter::ContinuousBatchArbiter;
use paddler_agent::continuous_batch_arbiter_build_outcome::ContinuousBatchArbiterBuildOutcome;
use paddler_agent::continuous_batch_arbiter_spawn_outcome::ContinuousBatchArbiterSpawnOutcome;
use paddler_agent::desired_model_resolution::DesiredModelResolution;
use paddler_agent::model_metadata_holder::ModelMetadataHolder;
use paddler_agent::resolve_desired_model::resolve_desired_model;
use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent::slot_aggregated_status_manager::SlotAggregatedStatusManager;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_tests::model_card::ModelCard;
use paddler_tests::model_card::qwen3_0_6b::qwen3_0_6b;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn arbiter_spawn_is_cancelled_when_shutdown_fires_before_the_model_loads() -> Result<()> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_0_6b();

    let model_path = match resolve_desired_model(
        &CancellationToken::new(),
        &AgentDesiredModel::HuggingFace(reference),
        Arc::new(SlotAggregatedStatus::new(1)),
    )
    .await?
    {
        DesiredModelResolution::Resolved(model_path) => model_path,
        other => return Err(anyhow!("qwen3 must resolve to a local path, got {other:?}")),
    };

    let applicable_state = AgentApplicableState {
        chat_template_override: None,
        inference_parameters: InferenceParameters {
            n_gpu_layers: gpu_layer_count,
            ..InferenceParameters::deterministic()
        },
        model_path: Some(model_path),
        multimodal_projection_path: None,
    };

    let arbiter = match ContinuousBatchArbiter::build_from_applicable_state(
        applicable_state,
        None,
        1,
        Arc::new(ModelMetadataHolder::new()),
        Arc::new(SlotAggregatedStatusManager::new(1)),
    ) {
        ContinuousBatchArbiterBuildOutcome::ReadyToSpawn(arbiter) => arbiter,
        ContinuousBatchArbiterBuildOutcome::NoModelConfigured => {
            return Err(anyhow!(
                "the arbiter must be ready to spawn with a resolved model"
            ));
        }
    };

    let cancellation_token = CancellationToken::new();

    cancellation_token.cancel();

    let outcome = arbiter.spawn(&cancellation_token).await?;

    assert!(
        matches!(outcome, ContinuousBatchArbiterSpawnOutcome::Cancelled),
        "a shutdown before the model finishes loading must cancel the spawn and cleanly join the loading thread"
    );

    Ok(())
}
