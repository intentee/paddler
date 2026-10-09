use std::mem::discriminant;
use std::sync::mpsc::channel;
use std::thread::spawn;

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::await_scheduler_startup::await_scheduler_startup;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_runtime::send_startup_signal::send_startup_signal;
use paddler_agent_runtime::startup_signal_delivery::StartupSignalDelivery;

use crate::number_request_preparer::NumberRequestPreparer;

#[tokio::test]
async fn a_cancelled_startup_joins_the_scheduler_thread() {
    let NumberRequestPreparer {
        request_preparer, ..
    } = NumberRequestPreparer::new();
    let (scheduler_ready_tx, scheduler_ready_rx) = oneshot::channel();
    let (signalled_tx, signalled_rx) = channel();
    let scheduler_thread_handle = spawn(move || -> Result<(), String> {
        assert!(matches!(
            send_startup_signal(scheduler_ready_tx, request_preparer),
            StartupSignalDelivery::Delivered
        ));
        signalled_tx
            .send(())
            .expect("the test must wait for the signal");

        Ok(())
    });
    let cancellation_token = CancellationToken::new();

    signalled_rx
        .recv()
        .expect("the scheduler thread must signal before the startup is awaited");
    cancellation_token.cancel();

    let spawn_outcome = await_scheduler_startup(
        &cancellation_token,
        scheduler_ready_rx,
        scheduler_thread_handle,
    )
    .await
    .expect("a cancelled startup must join the scheduler thread");

    assert_eq!(
        discriminant(&spawn_outcome),
        discriminant(&SchedulerSpawnOutcome::Cancelled)
    );
}
