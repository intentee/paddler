use tokio::sync::mpsc::unbounded_channel;

use crate::scheduler_ran_after_handover::scheduler_ran_after_handover;

#[test]
fn a_scheduler_whose_startup_was_abandoned_never_runs() {
    let (scheduler_startup_tx, scheduler_startup_rx) = unbounded_channel();

    drop(scheduler_startup_rx);

    assert!(!scheduler_ran_after_handover(&scheduler_startup_tx));
}
