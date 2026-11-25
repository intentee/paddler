#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::embedding_batch_generation::EmbeddingBatchGeneration;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_runtime_tests::embedding_request_preparer_under_test::EmbeddingRequestPreparerUnderTest;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_request_params_fixture::with_nul_byte::with_nul_byte;

#[tokio::test(flavor = "multi_thread")]
async fn requests_the_preparation_rejects_never_reach_the_scheduler() {
    let embedding_request_preparer = EmbeddingRequestPreparerUnderTest::load()
        .expect("the embedding request preparer must load its model");
    let EmbeddingBatchGeneration {
        agent_request,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        mut generated_embedding_rx,
    } = embedding_request_preparer.embedding_batch(vec![EmbeddingInputDocument {
        content: with_nul_byte("Document"),
        id: "untokenizable".to_owned(),
    }]);

    embedding_request_preparer
        .request_preparer
        .prepare(agent_request);
    embedding_request_preparer
        .request_preparer
        .stop_scheduler_after_pending_preparations()
        .await;

    assert!(matches!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::InputTokenizationFailed(_))
    ));
    assert!(matches!(
        embedding_request_preparer
            .drained_scheduler_messages()
            .as_slice(),
        [SchedulerMessage::Shutdown]
    ));
}
