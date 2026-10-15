use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::SendError;

use crate::continuation_decision::ContinuationDecision;
use crate::websocket_close_cause::WebSocketCloseCause;
use crate::websocket_outbound_frame::WebSocketOutboundFrame;

pub fn enqueue_websocket_frame(
    outbound_tx: &UnboundedSender<WebSocketOutboundFrame>,
    frame: WebSocketOutboundFrame,
) -> ContinuationDecision {
    match outbound_tx.send(frame) {
        Ok(()) => ContinuationDecision::Continue,
        Err(SendError(_unsent_frame)) => {
            ContinuationDecision::Stop(WebSocketCloseCause::SessionAlreadyClosed)
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::unbounded_channel;

    use super::enqueue_websocket_frame;
    use crate::continuation_decision::ContinuationDecision;
    use crate::websocket_close_cause::WebSocketCloseCause;
    use crate::websocket_outbound_frame::WebSocketOutboundFrame;

    #[test]
    fn a_frame_for_a_writer_that_stopped_ends_the_connection_as_already_closed() {
        let (outbound_tx, outbound_rx) = unbounded_channel();

        drop(outbound_rx);

        assert_eq!(
            enqueue_websocket_frame(&outbound_tx, WebSocketOutboundFrame::Ping),
            ContinuationDecision::Stop(WebSocketCloseCause::SessionAlreadyClosed)
        );
    }
}
