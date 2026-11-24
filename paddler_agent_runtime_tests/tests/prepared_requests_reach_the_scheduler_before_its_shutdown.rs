#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_embeddings::prepared_embedding_batch_request::PreparedEmbeddingBatchRequest;
use paddler_agent_pipeline_harness::embedding_batch_generation::EmbeddingBatchGeneration;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_runtime_tests::embedding_request_preparer_under_test::EmbeddingRequestPreparerUnderTest;
use paddler_request_params_fixture::numbered_embedding_documents::numbered_embedding_documents;

#[tokio::test(flavor = "multi_thread")]
async fn prepared_requests_reach_the_scheduler_before_its_shutdown() {
    let embedding_request_preparer = EmbeddingRequestPreparerUnderTest::load()
        .expect("the embedding request preparer must load its model");
    let EmbeddingBatchGeneration {
        agent_request,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        generated_embedding_rx: _generated_embedding_rx,
    } = embedding_request_preparer.embedding_batch(numbered_embedding_documents(1));

    embedding_request_preparer
        .request_preparer
        .prepare(agent_request);
    embedding_request_preparer
        .request_preparer
        .stop_scheduler_after_pending_preparations()
        .await;

    assert!(matches!(
        embedding_request_preparer.drained_scheduler_messages().as_slice(),
        [
            SchedulerMessage::Command(PreparedEmbeddingBatchRequest { inputs, .. }),
            SchedulerMessage::Shutdown,
        ] if inputs.len() == 1
    ));
}
