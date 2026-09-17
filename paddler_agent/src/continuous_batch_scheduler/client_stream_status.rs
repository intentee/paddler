#[derive(Debug, Eq, PartialEq)]
pub enum ClientStreamStatus {
    Open,
    Dropped,
}
