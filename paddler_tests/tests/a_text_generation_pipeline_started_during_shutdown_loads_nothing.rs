use std::collections::BTreeSet;
use std::mem::discriminant;
use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_pipeline::TextGenerationPipeline;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_started_during_shutdown_loads_nothing() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let agent_shutdown = CancellationToken::new();

    agent_shutdown.cancel();

    let spawn_outcome = TextGenerationPipeline {
        inference_runtime_context: inference_runtime_context.clone(),
        model_path: PathBuf::from("model-that-is-never-read.gguf"),
        model_runtime_parameters: ModelRuntimeParameters::default(),
        text_generation_settings: TextGenerationSettings {
            chat_template_source: ChatTemplateSource::EmbeddedInModel,
            image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
            multimodal_projection: MultimodalProjection::NotConfigured,
            sampling_parameters: SamplingParameters::default(),
        },
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
