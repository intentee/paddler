#![cfg(feature = "tests_that_use_llms")]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_agent::continuous_batch_arbiter::ContinuousBatchArbiter;
use paddler_agent::continuous_batch_arbiter_build_outcome::ContinuousBatchArbiterBuildOutcome;
use paddler_agent::model_metadata_holder::ModelMetadataHolder;
use paddler_agent::slot_aggregated_status_manager::SlotAggregatedStatusManager;
use paddler_messaging::inference_parameters::InferenceParameters;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_arbiter_fails_to_spawn_when_n_batch_exceeds_u32() -> Result<()> {
    let applicable_state = AgentApplicableState {
        chat_template_override: None,
        inference_parameters: InferenceParameters {
            n_batch: usize::MAX,
            ..InferenceParameters::default()
        },
        multimodal_projection_path: None,
        model_path: Some(PathBuf::from("/nonexistent/model.gguf")),
    };
    let model_metadata_holder = Arc::new(ModelMetadataHolder::new());
    let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(1));

    let outcome = ContinuousBatchArbiter::build_from_applicable_state(
        applicable_state,
        None,
        1,
        model_metadata_holder,
        slot_aggregated_status_manager,
    );

    let ContinuousBatchArbiterBuildOutcome::ReadyToSpawn(arbiter) = outcome else {
        return Err(anyhow!("expected ReadyToSpawn build outcome"));
    };

    let result = arbiter.spawn().await;

    assert!(result.is_err());

    Ok(())
}
