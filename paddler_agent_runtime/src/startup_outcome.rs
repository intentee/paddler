pub enum StartupOutcome<TStarted> {
    Cancelled,
    Ready(TStarted),
}
