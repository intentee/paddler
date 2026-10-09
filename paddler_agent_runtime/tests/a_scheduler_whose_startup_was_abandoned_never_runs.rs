use tokio::sync::oneshot;

use crate::scheduler_ran_after_handover::scheduler_ran_after_handover;

#[test]
fn a_scheduler_whose_startup_was_abandoned_never_runs() {
    let (scheduler_ready_tx, scheduler_ready_rx) = oneshot::channel();

    drop(scheduler_ready_rx);

    assert!(!scheduler_ran_after_handover(scheduler_ready_tx));
}
