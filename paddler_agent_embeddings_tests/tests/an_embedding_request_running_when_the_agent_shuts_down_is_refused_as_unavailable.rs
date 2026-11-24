#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_embeddings_tests::embedding_documents_fixture::EmbeddingDocumentsFixture;
use paddler_agent_pipeline_harness::model_serving_text_generation_and_embeddings::ModelServingTextGenerationAndEmbeddings;
use paddler_agent_runtime::shuts_down::ShutsDown as _;
use paddler_messaging::embedding_result::EmbeddingResult;

const DOCUMENT_COUNT: usize = 64;

#[tokio::test(flavor = "multi_thread")]
async fn an_embedding_request_running_when_the_agent_shuts_down_is_refused_as_unavailable() {
    let agent_shutdown = CancellationToken::new();
    let ModelServingTextGenerationAndEmbeddings {
        llama_runtime,
        served_model,
        ..
    } = ModelServingTextGenerationAndEmbeddings::start(&agent_shutdown)
        .await
        .expect("the model must load")
        .expect("text generation and embeddings must start");
    let EmbeddingDocumentsFixture {
        agent_request,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        generated_embedding_rx,
    } = EmbeddingDocumentsFixture::embedded_one_at_a_time(
        DOCUMENT_COUNT,
        llama_runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );

    served_model
        .capabilities
        .embeddings
        .forward(None, agent_request);
    agent_shutdown.cancel();

    assert!(matches!(
        EmbeddingDocumentsFixture::collect_results(generated_embedding_rx)
            .await
            .last(),
        Some(EmbeddingResult::SchedulerUnavailable(_))
    ));

    served_model.shut_down().await;
}
