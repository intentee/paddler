use std::collections::BTreeSet;
use std::mem::discriminant;
use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_decision::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_pipeline_started_during_shutdown_loads_nothing() {
    let inference_runtime_context = agent_inference_runtime_context(DECISION_SLOTS_MINIMUM);
    let agent_shutdown = CancellationToken::new();

    agent_shutdown.cancel();

    let spawn_outcome = DecisionPipeline {
        inference_runtime_context: inference_runtime_context.clone(),
        model_path: PathBuf::from("model-that-is-never-read.gguf"),
        model_runtime_parameters: ModelRuntimeParameters::default(),
        pointer_head_path: PathBuf::from("pointer-head-that-is-never-read.gguf"),
    }
    .spawn(&agent_shutdown)
    .await
    .expect("a pipeline started during shutdown must not attempt to start");

    assert_eq!(
        discriminant(&spawn_outcome),
        discriminant(&SchedulerSpawnOutcome::Cancelled)
    );
    assert_eq!(
        inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status
            .issues,
        BTreeSet::new()
    );
}
