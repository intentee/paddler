use std::fmt::Display;
use std::sync::Arc;

use futures_util::Sink;
use futures_util::SinkExt;
use log::error;
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::inference_socket::inference_socket_exchange::InferenceSocketExchange;
use crate::inference_socket::outbound_frames::OutboundFrames;

pub fn spawn_write_task<TWebSocketWriteSink>(
    ws_write: TWebSocketWriteSink,
    exchange: Arc<InferenceSocketExchange>,
) -> JoinHandle<()>
where
    TWebSocketWriteSink: Sink<WsMessage> + Send + Unpin + 'static,
    TWebSocketWriteSink::Error: Display,
{
    spawn(async move {
        let mut ws_write = ws_write;

        while let OutboundFrames::Queued(frames) = exchange.next_outbound_frames().await {
            for frame in frames {
                if let Err(write_error) = ws_write.send(WsMessage::Text(frame.into())).await {
                    error!("WebSocket write error: {write_error}");
                    exchange.close();

                    return;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tokio::sync::mpsc::channel;
    use tokio_tungstenite::tungstenite::Message as WsMessage;
    use tokio_util::sync::PollSender;

    use super::spawn_write_task;
    use crate::error::Error;
    use crate::inference_socket::inference_socket_exchange::InferenceSocketExchange;
    use crate::inference_socket::outbound_request::OutboundRequest;

    #[tokio::test]
    async fn a_failed_write_drops_every_awaiting_request() {
        let exchange = Arc::new(InferenceSocketExchange::default());
        let (frames_tx, frames_rx) = channel::<WsMessage>(1);
        let mut response_rx = exchange
            .register(
                OutboundRequest::serialize("request-1".to_owned(), &json!({}))
                    .expect("a JSON request must serialize"),
            )
            .expect("an open exchange must accept a request");

        drop(frames_rx);

        spawn_write_task(PollSender::new(frames_tx), exchange)
            .await
            .expect("the write task must not panic");

        assert!(matches!(
            response_rx.recv().await,
            Some(Err(Error::ConnectionDropped { request_id })) if request_id == "request-1"
        ));
    }
}
