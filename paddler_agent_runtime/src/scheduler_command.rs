pub trait SchedulerCommand: Send + 'static {
    fn reject_because_the_scheduler_stopped(self, agent_name: Option<&str>);
}
