use futures_util::Sink;
use futures_util::SinkExt as _;
use tokio::select;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use crate::frame_writing_end::FrameWritingEnd;

pub async fn write_frames_until_connection_closes<TFrame, TSink>(
    connection_close: CancellationToken,
    mut outbound_rx: UnboundedReceiver<TFrame>,
    mut sink: TSink,
) -> FrameWritingEnd<TSink::Error>
where
    TSink: Sink<TFrame> + Unpin,
{
    while let Some(frame) = outbound_rx.recv().await {
        select! {
            biased;
            send_outcome = sink.send(frame) => {
                if let Err(send_error) = send_outcome {
                    return FrameWritingEnd::SendFailed(send_error);
                }
            }
            () = connection_close.cancelled() => return FrameWritingEnd::ConnectionClosed,
        }
    }

    FrameWritingEnd::EveryFrameSent
}

#[cfg(test)]
mod tests {
    use futures::channel::mpsc as sink_channel;
    use tokio::spawn;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::write_frames_until_connection_closes;
    use crate::frame_writing_end::FrameWritingEnd;

    #[tokio::test]
    async fn a_send_its_peer_never_completes_is_abandoned_once_the_connection_closes() {
        let (sink, _undrained_sink_receiver) = sink_channel::channel::<u32>(0);
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let connection_close = CancellationToken::new();
        let writer = spawn(write_frames_until_connection_closes(
            connection_close.clone(),
            outbound_rx,
            sink,
        ));

        outbound_tx.send(1).unwrap();
        outbound_tx.send(2).unwrap();
        connection_close.cancel();

        assert_eq!(writer.await.unwrap(), FrameWritingEnd::ConnectionClosed);
    }

    #[tokio::test]
    async fn a_frame_the_sink_rejects_stops_the_writer() {
        let (sink, sink_receiver) = sink_channel::unbounded::<u32>();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        drop(sink_receiver);
        outbound_tx.send(1).unwrap();

        assert!(matches!(
            write_frames_until_connection_closes(CancellationToken::new(), outbound_rx, sink).await,
            FrameWritingEnd::SendFailed(send_error) if send_error.is_disconnected()
        ));
    }

    #[tokio::test]
    async fn a_writer_sends_every_queued_frame_until_its_queue_closes() {
        let (sink, mut sink_receiver) = sink_channel::unbounded::<u32>();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        outbound_tx.send(1).unwrap();
        outbound_tx.send(2).unwrap();
        drop(outbound_tx);

        assert_eq!(
            write_frames_until_connection_closes(CancellationToken::new(), outbound_rx, sink).await,
            FrameWritingEnd::EveryFrameSent
        );
        assert_eq!(sink_receiver.try_recv().unwrap(), 1);
        assert_eq!(sink_receiver.try_recv().unwrap(), 2);
    }
}
