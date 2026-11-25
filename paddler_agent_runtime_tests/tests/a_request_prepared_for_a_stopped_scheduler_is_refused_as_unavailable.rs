#![cfg(feature = "tests_that_use_llms")]

use log::Level;
use tokio::sync::mpsc::error::TryRecvError;

use paddler_agent_pipeline_harness::embedding_batch_generation::EmbeddingBatchGeneration;
use paddler_agent_runtime_tests::embedding_request_preparer_under_test::EmbeddingRequestPreparerUnderTest;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_request_params_fixture::numbered_embedding_documents::numbered_embedding_documents;

#[tokio::test(flavor = "multi_thread")]
async fn a_request_prepared_for_a_stopped_scheduler_is_refused_as_unavailable() {
    CapturingLogger::install();

    let embedding_request_preparer = EmbeddingRequestPreparerUnderTest::load()
        .expect("the embedding request preparer must load its model");
    let EmbeddingBatchGeneration {
        agent_request,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        mut generated_embedding_rx,
    } = embedding_request_preparer.embedding_batch(numbered_embedding_documents(1));
    let EmbeddingRequestPreparerUnderTest {
        llama_runtime: _llama_runtime,
        request_preparer,
        scheduler_message_rx,
    } = embedding_request_preparer;

    drop(scheduler_message_rx);

    request_preparer.prepare(agent_request);
    request_preparer
        .stop_scheduler_after_pending_preparations()
        .await;

    assert!(matches!(
        generated_embedding_rx.try_recv(),
        Ok(EmbeddingResult::SchedulerUnavailable(_))
    ));
    assert_eq!(
        generated_embedding_rx.try_recv(),
        Err(TryRecvError::Disconnected)
    );
    assert!(
        CapturingLogger::records_for("paddler_agent_runtime::scheduler_request_preparer")
            .iter()
            .any(|CapturedRecord { level, .. }| *level == Level::Info)
    );
}
