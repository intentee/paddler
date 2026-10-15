use crate::scheduler_command::SchedulerCommand;

pub trait PreparesSchedulerCommand: Send + Sync + 'static {
    type Command: SchedulerCommand;
    type Request: Send + 'static;

    fn prepare_scheduler_command(
        &self,
        agent_name: Option<&str>,
        request: Self::Request,
    ) -> Option<Self::Command>;
}
