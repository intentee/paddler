use tokio::sync::mpsc::unbounded_channel;

use crate::scheduler_ran_after_handover::scheduler_ran_after_handover;

#[test]
fn a_scheduler_whose_startup_was_delivered_runs() {
    let (scheduler_startup_tx, mut scheduler_startup_rx) = unbounded_channel();

    assert!(scheduler_ran_after_handover(&scheduler_startup_tx));
    assert_eq!(scheduler_startup_rx.try_recv(), Ok(()));
}
