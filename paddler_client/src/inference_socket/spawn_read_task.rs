use std::sync::Arc;

use futures_util::Stream;
use futures_util::StreamExt;
use log::error;
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::inference_socket::cluster_inference_mode_broadcaster::ClusterInferenceModeBroadcaster;
use crate::inference_socket::inbound_message_router::InboundMessageRouter;
use crate::inference_socket::pending_requests::PendingRequests;

#[must_use]
pub fn spawn_read_task<TWebSocketReadStream>(
    ws_read: TWebSocketReadStream,
    pending: Arc<PendingRequests>,
    cluster_inference_mode_broadcaster: Arc<ClusterInferenceModeBroadcaster>,
) -> JoinHandle<()>
where
    TWebSocketReadStream: Stream<Item = Result<WsMessage, WebSocketError>> + Send + Unpin + 'static,
{
    spawn(async move {
        let mut ws_read = ws_read;
        let router = InboundMessageRouter {
            cluster_inference_mode_broadcaster,
            pending,
        };

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::StreamExt as _;
    use futures_util::stream::iter;
    use futures_util::stream::pending;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    use super::spawn_read_task;
    use crate::error::Error;
    use crate::inference_socket::cluster_inference_mode_broadcaster::ClusterInferenceModeBroadcaster;
    use crate::inference_socket::pending_requests::PendingRequests;

    #[tokio::test]
    async fn a_binary_frame_drops_every_pending_request_without_waiting_for_more_frames() {
        let pending_requests = Arc::new(PendingRequests::default());
        let mut response_rx = pending_requests
            .register("request-1".to_owned())
            .expect("an open connection must accept a request");

        spawn_read_task(
            iter([Ok(WsMessage::Binary(vec![0].into()))]).chain(pending()),
            pending_requests,
            Arc::new(ClusterInferenceModeBroadcaster::default()),
        )
        .await
        .expect("the read task must not panic");

        assert!(matches!(
            response_rx.recv().await,
            Some(Err(Error::ConnectionDropped { request_id })) if request_id == "request-1"
        ));
    }
}
