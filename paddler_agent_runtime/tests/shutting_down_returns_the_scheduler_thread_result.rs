use std::thread::spawn;

use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;

use crate::number_request_preparer::NumberRequestPreparer;

#[tokio::test]
async fn shutting_down_returns_the_scheduler_thread_result() {
    let NumberRequestPreparer {
        request_preparer,
        scheduler_message_rx,
        ..
    } = NumberRequestPreparer::new();
    let scheduler_thread_handle = spawn(move || {
        scheduler_message_rx
            .iter()
            .find(|scheduler_message| matches!(scheduler_message, SchedulerMessage::Shutdown))
            .map_or(
                Ok(()),
                |_shutdown| Err("drained before stopping".to_owned()),
            )
    });

    let shutdown_result = SchedulerHandle {
        request_preparer,
        scheduler_thread_handle,
    }
    .shut_down()
    .await;

    assert_eq!(shutdown_result, Err("drained before stopping".to_owned()));
}
