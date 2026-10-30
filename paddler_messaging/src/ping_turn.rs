#[derive(Debug, Eq, PartialEq)]
pub enum PingTurn {
    PeerStoppedAnswering,
    SendPing,
}
