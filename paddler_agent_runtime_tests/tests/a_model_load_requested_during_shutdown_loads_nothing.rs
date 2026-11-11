use std::collections::BTreeSet;
use std::mem::discriminant;
use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::llama_model_loader::LlamaModelLoader;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::startup_outcome::StartupOutcome;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;

#[tokio::test]
async fn a_model_load_requested_during_shutdown_loads_nothing() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let agent_shutdown = CancellationToken::new();

    agent_shutdown.cancel();

    let load_outcome = LlamaModelLoader {
        inference_runtime_context: inference_runtime_context.clone(),
        llama_model_settings: LlamaModelSettings {
            model_path: PathBuf::from("model-that-is-never-read.gguf"),
            n_gpu_layers: 0,
        },
    }
    .load_unless_shutting_down(&agent_shutdown)
    .await
    .expect("a model load requested during shutdown must not attempt to load");

    assert_eq!(
        discriminant(&load_outcome),
        discriminant(&StartupOutcome::Cancelled)
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
