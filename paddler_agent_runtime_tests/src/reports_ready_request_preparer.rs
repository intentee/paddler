use std::sync::Arc;

use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::startup_outcome::StartupOutcome;

pub trait ReportsReadyRequestPreparer<TPreparation: PreparesSchedulerCommand> {
    fn ready_request_preparer(&self) -> Option<&Arc<SchedulerRequestPreparer<TPreparation>>>;
}

impl<TPreparation: PreparesSchedulerCommand> ReportsReadyRequestPreparer<TPreparation>
    for StartupOutcome<SchedulerHandle<TPreparation>>
{
    fn ready_request_preparer(&self) -> Option<&Arc<SchedulerRequestPreparer<TPreparation>>> {
        match self {
            Self::Cancelled => None,
            Self::Ready(SchedulerHandle {
                request_preparer, ..
            }) => Some(request_preparer),
        }
    }
}
