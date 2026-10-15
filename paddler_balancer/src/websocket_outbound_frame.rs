use actix_ws::CloseReason;
use bytes::Bytes;

#[derive(Debug, Eq, PartialEq)]
pub enum WebSocketOutboundFrame {
    Close(Option<CloseReason>),
    Ping,
    Pong(Bytes),
    Text(String),
}
