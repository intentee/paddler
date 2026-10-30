#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::shuts_down::ShutsDown as _;
use paddler_messaging::embedding_result::EmbeddingResult;

use crate::embedding_documents_fixture::EmbeddingDocumentsFixture;
use crate::kev_model_serving_without_decisions::KevModelServingWithoutDecisions;

const DOCUMENT_COUNT: usize = 64;

#[tokio::test(flavor = "multi_thread")]
async fn an_embedding_request_accepted_before_its_scheduler_stops_is_finished() {
    let KevModelServingWithoutDecisions {
        llama_runtime,
        served_model,
        ..
    } = KevModelServingWithoutDecisions::start(&CancellationToken::new()).await;
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
        .request_preparer
        .prepare(agent_request);
    served_model.shut_down().await;

    let embedding_results =
        EmbeddingDocumentsFixture::collect_results(generated_embedding_rx).await;

    assert_eq!(embedding_results.len(), DOCUMENT_COUNT + 1);
    assert_eq!(embedding_results.last(), Some(&EmbeddingResult::Done));
}
