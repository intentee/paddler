use log::Level;
use tokio::sync::mpsc::error::TryRecvError;

use paddler_agent_runtime_tests::number_request_preparer::NumberRequestPreparer;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;

#[tokio::test]
async fn commands_for_a_stopped_scheduler_are_rejected() {
    CapturingLogger::install();

    let NumberRequestPreparer {
        mut rejected_rx,
        request_preparer,
        scheduler_message_rx,
    } = NumberRequestPreparer::default();

    drop(scheduler_message_rx);

    request_preparer.prepare(4);
    request_preparer
        .stop_scheduler_after_pending_preparations()
        .await;

    assert_eq!(rejected_rx.try_recv(), Ok(4));
    assert_eq!(rejected_rx.try_recv(), Err(TryRecvError::Empty));
    assert!(
        CapturingLogger::records_for("paddler_agent_runtime::scheduler_request_preparer")
            .iter()
            .any(|CapturedRecord { level, .. }| *level == Level::Info)
    );
}
