use std::mem::discriminant;
use std::sync::mpsc::channel;
use std::thread::spawn;

use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_messaging::model_metadata::ModelMetadata;

use crate::number_command::NumberCommand;
use crate::number_request_preparer::NumberRequestPreparer;

#[tokio::test]
async fn shutting_down_stops_the_scheduler_and_waits_for_its_thread() {
    let NumberRequestPreparer {
        request_preparer,
        scheduler_message_rx,
        ..
    } = NumberRequestPreparer::new();
    let (shutdown_observed_tx, shutdown_observed_rx) = channel();
    let scheduler_thread_handle = spawn(move || {
        shutdown_observed_tx
            .send(
                scheduler_message_rx
                    .recv()
                    .map(|scheduler_message| discriminant(&scheduler_message)),
            )
            .expect("the test must wait for the scheduler thread");
    });

    SchedulerHandle {
        model_metadata: ModelMetadata::default(),
        request_preparer,
        scheduler_thread_handle,
    }
    .shut_down()
    .await;

    assert_eq!(
        shutdown_observed_rx.try_recv(),
        Ok(Ok(discriminant(
            &SchedulerMessage::<NumberCommand>::Shutdown
        )))
    );
}
