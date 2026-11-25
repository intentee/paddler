#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_pipeline_harness::model_serving_text_generation_and_embeddings::ModelServingTextGenerationAndEmbeddings;
use paddler_agent_runtime::shuts_down::ShutsDown as _;

#[tokio::test(flavor = "multi_thread")]
async fn shutting_down_a_served_model_stops_its_schedulers_and_frees_their_contexts() {
    let ModelServingTextGenerationAndEmbeddings {
        llama_runtime,
        served_model,
        ..
    } = ModelServingTextGenerationAndEmbeddings::start(&CancellationToken::new())
        .await
        .expect("the model must load")
        .expect("text generation and embeddings must start");
    let vram_residencies = llama_runtime
        .inference_runtime_context
        .vram_residencies
        .clone();

    served_model.shut_down().await;

    assert!(vram_residencies.embeddings_context.try_occupy().is_ok());
    assert!(
        vram_residencies
            .text_generation_context
            .try_occupy()
            .is_ok()
    );
}
