use tokio::sync::mpsc::UnboundedSender;

use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;

pub fn scheduler_ran_after_handover(scheduler_startup_tx: &UnboundedSender<()>) -> bool {
    let mut scheduler_ran = false;

    run_scheduler_unless_startup_abandoned(scheduler_startup_tx, (), || scheduler_ran = true);

    scheduler_ran
}
