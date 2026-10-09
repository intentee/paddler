use std::mem::discriminant;
use std::thread::spawn;

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::await_scheduler_startup::await_scheduler_startup;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_runtime::send_startup_signal::send_startup_signal;
use paddler_agent_runtime::startup_signal_delivery::StartupSignalDelivery;

use crate::number_request_preparer::NumberRequestPreparer;

#[tokio::test]
async fn a_scheduler_that_signals_readiness_starts() {
    let NumberRequestPreparer {
        request_preparer, ..
    } = NumberRequestPreparer::new();
    let (scheduler_ready_tx, scheduler_ready_rx) = oneshot::channel();
    let scheduler_thread_handle = spawn(move || -> Result<(), String> {
        assert!(matches!(
            send_startup_signal(scheduler_ready_tx, request_preparer),
            StartupSignalDelivery::Delivered
        ));

        Ok(())
    });

    let spawn_outcome = await_scheduler_startup(
        &CancellationToken::new(),
        scheduler_ready_rx,
        scheduler_thread_handle,
    )
    .await
    .expect("a scheduler that signalled readiness must start");

    assert_ne!(
        discriminant(&spawn_outcome),
        discriminant(&SchedulerSpawnOutcome::Cancelled)
    );
}
