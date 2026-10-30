use log::error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::management_connection_end::ManagementConnectionEnd;
use crate::management_connection_step::ManagementConnectionStep;

pub fn enqueue_outbound_frame(
    outbound_tx: &mpsc::UnboundedSender<Message>,
    frame: Message,
    step_after_enqueue: ManagementConnectionStep,
) -> ManagementConnectionStep {
    match outbound_tx.send(frame) {
        Ok(()) => step_after_enqueue,
        Err(SendError(undelivered_frame)) => {
            error!("The management socket writer stopped before it could send {undelivered_frame}");

            ManagementConnectionStep::End(ManagementConnectionEnd::OutgoingStreamFailed)
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::Bytes;
    use tokio_tungstenite::tungstenite::protocol::Message;

    use super::enqueue_outbound_frame;
    use crate::management_connection_end::ManagementConnectionEnd;
    use crate::management_connection_step::ManagementConnectionStep;

    #[test]
    fn ends_the_connection_when_the_writer_stopped() {
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        drop(outbound_rx);

        assert!(matches!(
            enqueue_outbound_frame(
                &outbound_tx,
                Message::Ping(Bytes::new()),
                ManagementConnectionStep::Continue,
            ),
            ManagementConnectionStep::End(management_connection_end)
                if management_connection_end == ManagementConnectionEnd::OutgoingStreamFailed
        ));
    }
}
