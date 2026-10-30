use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;
use paddler_agent_runtime::scheduler_ready::SchedulerReady;
use paddler_agent_runtime::start_scheduler_thread::start_scheduler_thread;

use crate::number_request_preparer::NumberRequestPreparer;
use crate::reports_ready_request_preparer::ReportsReadyRequestPreparer as _;

#[tokio::test]
async fn a_scheduler_that_signals_readiness_starts() {
    let NumberRequestPreparer {
        request_preparer, ..
    } = NumberRequestPreparer::new();
    let signalled_request_preparer = request_preparer.clone();

    let spawn_outcome = start_scheduler_thread::<_, String, _>(
        &CancellationToken::new(),
        move |_agent_shutdown, scheduler_startup_tx| {
            run_scheduler_unless_startup_abandoned(
                scheduler_startup_tx,
                Ok(SchedulerReady { request_preparer }),
                || {},
            );

            Ok(())
        },
    )
    .await
    .expect("a scheduler that signalled readiness must start");

    assert!(
        spawn_outcome
            .ready_request_preparer()
            .is_some_and(|ready_request_preparer| Arc::ptr_eq(
                ready_request_preparer,
                &signalled_request_preparer
            ))
    );
}
