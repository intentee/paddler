pub enum SchedulerMessage<TCommand> {
    Command(TCommand),
    Shutdown,
}
