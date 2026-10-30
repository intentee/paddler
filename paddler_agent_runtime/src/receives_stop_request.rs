use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::error::TryRecvError;

pub trait ReceivesStopRequest {
    fn is_stop_requested(&mut self) -> bool;
}

impl ReceivesStopRequest for UnboundedReceiver<()> {
    fn is_stop_requested(&mut self) -> bool {
        match self.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => true,
            Err(TryRecvError::Empty) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::unbounded_channel;

    use super::ReceivesStopRequest as _;

    #[test]
    fn a_sent_stop_request_is_received() {
        let (stop_tx, mut stop_rx) = unbounded_channel();

        stop_tx.send(()).unwrap();

        assert!(stop_rx.is_stop_requested());
    }

    #[test]
    fn a_requester_that_went_away_requests_a_stop() {
        let (stop_tx, mut stop_rx) = unbounded_channel::<()>();

        drop(stop_tx);

        assert!(stop_rx.is_stop_requested());
    }

    #[test]
    fn a_waiting_requester_does_not_request_a_stop() {
        let (_stop_tx, mut stop_rx) = unbounded_channel::<()>();

        assert!(!stop_rx.is_stop_requested());
    }
}
