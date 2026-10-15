use std::fmt::Display;

use futures_util::Sink;
use futures_util::SinkExt as _;
use log::error;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;

pub async fn forward_outbound_frames<TSink>(
    mut sink: TSink,
    mut outbound_rx: mpsc::UnboundedReceiver<Message>,
) where
    TSink: Sink<Message> + Unpin,
    TSink::Error: Display,
{
    while let Some(frame) = outbound_rx.recv().await {
        if let Err(send_error) = sink.send(frame).await {
            error!("Failed to send a management socket frame: {send_error}");

            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::channel::mpsc as sink_channel;
    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::Bytes;
    use tokio_tungstenite::tungstenite::protocol::Message;

    use super::forward_outbound_frames;

    #[tokio::test]
    async fn a_frame_the_socket_rejects_stops_the_writer() {
        let (sink, sink_receiver) = sink_channel::unbounded::<Message>();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        drop(sink_receiver);
        outbound_tx.send(Message::Ping(Bytes::new())).unwrap();

        forward_outbound_frames(sink, outbound_rx).await;

        assert!(outbound_tx.is_closed());
    }
}
