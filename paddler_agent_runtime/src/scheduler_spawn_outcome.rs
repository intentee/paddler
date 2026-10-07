use crate::prepares_scheduler_command::PreparesSchedulerCommand;
use crate::scheduler_handle::SchedulerHandle;

pub enum SchedulerSpawnOutcome<TPreparation: PreparesSchedulerCommand, TError> {
    Cancelled,
    Ready(SchedulerHandle<TPreparation, TError>),
}
