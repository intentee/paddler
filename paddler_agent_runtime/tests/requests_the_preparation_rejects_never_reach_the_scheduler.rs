use std::mem::discriminant;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;

use crate::number_command::NumberCommand;
use crate::number_request_preparer::NumberRequestPreparer;

#[tokio::test]
async fn requests_the_preparation_rejects_never_reach_the_scheduler() {
    let NumberRequestPreparer {
        request_preparer,
        scheduler_message_rx,
        ..
    } = NumberRequestPreparer::new();

    request_preparer.prepare(1);
    request_preparer
        .stop_scheduler_after_pending_preparations()
        .await;

    let received: Vec<_> = scheduler_message_rx
        .try_iter()
        .map(|scheduler_message| discriminant(&scheduler_message))
        .collect();

    assert_eq!(
        received,
        vec![discriminant(&SchedulerMessage::<NumberCommand>::Shutdown)]
    );
}
