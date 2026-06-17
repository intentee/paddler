use std::fmt::Display;

use futures_util::Sink;
use futures_util::SinkExt;
use log::error;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub fn spawn_write_task<TWebSocketSink>(
    ws_write: TWebSocketSink,
    write_rx: UnboundedReceiver<String>,
) -> JoinHandle<()>
where
    TWebSocketSink: Sink<WsMessage> + Send + Unpin + 'static,
    TWebSocketSink::Error: Display,
{
    tokio::spawn(async move {
        let mut ws_write = ws_write;
        let mut write_rx = write_rx;

        while let Some(message) = write_rx.recv().await {
            if let Err(err) = ws_write.send(WsMessage::Text(message.into())).await {
                error!("WebSocket write error: {err}");
                break;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use futures_util::sink;
    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tokio_util::sync::PollSender;

    use super::spawn_write_task;

    fn enable_logging() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    }

    #[tokio::test]
    async fn forwards_queued_messages_then_stops_when_the_channel_closes() {
        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        write_tx.send("hello".to_owned()).expect("message queued");
        drop(write_tx);

        spawn_write_task(sink::drain::<WsMessage>(), write_rx)
            .await
            .expect("write task joins");
    }

    #[tokio::test]
    async fn stops_when_the_socket_sink_errors() {
        enable_logging();

        let (sink_tx, sink_rx) = mpsc::channel::<WsMessage>(1);

        drop(sink_rx);

        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        write_tx.send("hello".to_owned()).expect("message queued");

        spawn_write_task(PollSender::new(sink_tx), write_rx)
            .await
            .expect("write task joins");
    }
}
