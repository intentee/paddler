use paddler_agent_runtime::scheduler_message::SchedulerMessage;

use crate::number_command::NumberCommand;
use crate::number_request_preparer::NumberRequestPreparer;

#[tokio::test]
async fn prepared_requests_reach_the_scheduler_before_its_shutdown() {
    let NumberRequestPreparer {
        request_preparer,
        scheduler_message_rx,
        ..
    } = NumberRequestPreparer::new();

    request_preparer.prepare(2);
    request_preparer
        .stop_scheduler_after_pending_preparations()
        .await;

    let received: Vec<Option<u32>> = scheduler_message_rx
        .try_iter()
        .map(|scheduler_message| match scheduler_message {
            SchedulerMessage::Command(NumberCommand { value, .. }) => Some(value),
            SchedulerMessage::Shutdown => None,
        })
        .collect();

    assert_eq!(received, vec![Some(2), None]);
}
