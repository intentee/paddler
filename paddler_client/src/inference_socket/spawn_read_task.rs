use std::sync::Arc;

use futures_util::StreamExt;
use futures_util::stream::SplitStream;
use log::error;
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::inference_socket::inbound_message_router::InboundMessageRouter;
use crate::inference_socket::pending_requests::PendingRequests;

type WebSocketReadStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

#[must_use]
pub fn spawn_read_task(
    ws_read: WebSocketReadStream,
    pending: Arc<PendingRequests>,
) -> JoinHandle<()> {
    spawn(async move {
        let mut ws_read = ws_read;
        let router = InboundMessageRouter { pending };

        while let Some(msg_result) = ws_read.next().await {
            match msg_result {
                Ok(WsMessage::Text(text)) => router.route_text(&text),
                Ok(WsMessage::Close(_)) => break,
                Ok(WsMessage::Ping(_) | WsMessage::Pong(_)) => {}
                Ok(WsMessage::Binary(_) | WsMessage::Frame(_)) => {
                    error!(
                        "Closing the inference socket after a binary message it cannot interpret"
                    );
                    break;
                }
                Err(err) => {
                    error!("WebSocket read error: {err}");
                    break;
                }
            }
        }

        router.pending.close();
    })
}
