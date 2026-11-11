use actix_ws::CloseReason;
use bytes::Bytes;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::SendError;

use crate::continuation_decision::ContinuationDecision;
use crate::websocket_close_cause::WebSocketCloseCause;

#[derive(Debug, Eq, PartialEq)]
pub enum WebSocketOutboundFrame {
    Close(Option<CloseReason>),
    Ping,
    Pong(Bytes),
    Text(String),
}

impl WebSocketOutboundFrame {
    pub fn enqueue(self, outbound_tx: &UnboundedSender<Self>) -> ContinuationDecision {
        match outbound_tx.send(self) {
            Ok(()) => ContinuationDecision::Continue,
            Err(SendError(_unsent_frame)) => {
                ContinuationDecision::Stop(WebSocketCloseCause::SessionAlreadyClosed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::unbounded_channel;

    use super::WebSocketOutboundFrame;
    use crate::continuation_decision::ContinuationDecision;
    use crate::websocket_close_cause::WebSocketCloseCause;

    #[test]
    fn a_frame_for_a_writer_that_stopped_ends_the_connection_as_already_closed() {
        let (outbound_tx, outbound_rx) = unbounded_channel();

        drop(outbound_rx);

        assert_eq!(
            WebSocketOutboundFrame::Ping.enqueue(&outbound_tx),
            ContinuationDecision::Stop(WebSocketCloseCause::SessionAlreadyClosed)
        );
    }
}
