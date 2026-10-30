#[derive(Debug, Eq, PartialEq)]
pub enum OutboundFrames {
    ExchangeClosed,
    Queued(Vec<String>),
}
