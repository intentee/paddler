use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_messaging::model_metadata::ModelMetadata;

pub trait ReportsReadyModelMetadata {
    fn ready_model_metadata(&self) -> Option<&ModelMetadata>;
}

impl<TPreparation: PreparesSchedulerCommand> ReportsReadyModelMetadata
    for SchedulerSpawnOutcome<TPreparation>
{
    fn ready_model_metadata(&self) -> Option<&ModelMetadata> {
        match self {
            Self::Cancelled => None,
            Self::Ready(SchedulerHandle { model_metadata, .. }) => Some(model_metadata),
        }
    }
}
