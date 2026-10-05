#[derive(Debug, Eq, PartialEq)]
pub enum ReceiveStreamStopOutcome {
    RequestAlreadyFinished,
    StopSignalled,
}
