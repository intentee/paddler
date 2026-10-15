use tokio::sync::mpsc::error::TryRecvError;

use crate::number_request_preparer::NumberRequestPreparer;

#[tokio::test]
async fn commands_for_a_stopped_scheduler_are_rejected() {
    let NumberRequestPreparer {
        mut rejected_rx,
        request_preparer,
        scheduler_message_rx,
    } = NumberRequestPreparer::new();

    drop(scheduler_message_rx);

    request_preparer.prepare(4);
    request_preparer
        .stop_scheduler_after_pending_preparations()
        .await;

    assert_eq!(rejected_rx.try_recv(), Ok(4));
    assert_eq!(rejected_rx.try_recv(), Err(TryRecvError::Empty));
}
