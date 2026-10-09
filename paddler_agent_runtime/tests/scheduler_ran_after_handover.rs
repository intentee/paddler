use tokio::sync::oneshot;

use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;

pub fn scheduler_ran_after_handover(scheduler_ready_tx: oneshot::Sender<()>) -> bool {
    let mut scheduler_ran = false;

    run_scheduler_unless_startup_abandoned(scheduler_ready_tx, (), || scheduler_ran = true);

    scheduler_ran
}
