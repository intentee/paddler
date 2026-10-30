#[derive(Debug, Eq, PartialEq)]
pub enum FrameWritingEnd<TSendError> {
    ConnectionClosed,
    EveryFrameSent,
    SendFailed(TSendError),
}
