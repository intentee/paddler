use crate::websocket_close_cause::WebSocketCloseCause;

pub enum ContinuationDecision {
    Continue,
    Stop(WebSocketCloseCause),
}
