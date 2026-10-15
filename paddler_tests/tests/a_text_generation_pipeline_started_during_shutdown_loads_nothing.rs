use std::collections::BTreeSet;
use std::mem::discriminant;
use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_text_generation::llama_cpp_media_marker::LlamaCppMediaMarker;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_started_during_shutdown_loads_nothing() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let agent_shutdown = CancellationToken::new();

    agent_shutdown.cancel();

    let spawn_outcome = TextGenerationPipelineParts {
        chat_template_source: ChatTemplateSource::EmbeddedInModel,
        inference_runtime_context: inference_runtime_context.clone(),
        media_marker: LlamaCppMediaMarker,
        model_path: PathBuf::from("model-that-is-never-read.gguf"),
        multimodal_projection: MultimodalProjection::NotConfigured,
    }
    .into_text_generation_pipeline()
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
