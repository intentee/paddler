use tokio::sync::oneshot;

use crate::scheduler_ran_after_handover::scheduler_ran_after_handover;

#[test]
fn a_scheduler_whose_startup_was_delivered_runs() {
    let (scheduler_ready_tx, mut scheduler_ready_rx) = oneshot::channel();

    assert!(scheduler_ran_after_handover(scheduler_ready_tx));
    assert_eq!(scheduler_ready_rx.try_recv(), Ok(()));
}
