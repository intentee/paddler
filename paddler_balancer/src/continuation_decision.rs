use crate::websocket_close_cause::WebSocketCloseCause;

#[derive(Debug, Eq, PartialEq)]
pub enum ContinuationDecision {
    Continue,
    Stop(WebSocketCloseCause),
}
