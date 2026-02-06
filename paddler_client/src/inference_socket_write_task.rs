use futures_util::SinkExt;
use futures_util::stream::SplitSink;
use log::error;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

type WebSocketWriteSink =
    SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, WsMessage>;

pub fn spawn_inference_socket_write_task(
    ws_write: WebSocketWriteSink,
    write_rx: UnboundedReceiver<String>,
) -> JoinHandle<()> {
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
