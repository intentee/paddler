use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_handle::SchedulerHandle;

pub enum SchedulerSpawnOutcome<TPreparation: PreparesSchedulerCommand> {
    Cancelled,
    Ready(SchedulerHandle<TPreparation>),
}
