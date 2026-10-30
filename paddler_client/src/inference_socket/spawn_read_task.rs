use futures_util::Stream;
use futures_util::StreamExt;
use log::error;
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::inference_socket::inbound_message_router::InboundMessageRouter;

#[must_use]
pub fn spawn_read_task<TWebSocketReadStream>(
    ws_read: TWebSocketReadStream,
    router: InboundMessageRouter,
) -> JoinHandle<()>
where
    TWebSocketReadStream: Stream<Item = Result<WsMessage, WebSocketError>> + Send + Unpin + 'static,
{
    spawn(async move {
        let mut ws_read = ws_read;

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

        router.exchange.close();
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::StreamExt as _;
    use futures_util::stream::iter;
    use futures_util::stream::pending;
    use serde_json::json;
    use tokio_tungstenite::tungstenite::Bytes;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::spawn_read_task;
    use crate::error::Error;
    use crate::inference_socket::inbound_message_router::InboundMessageRouter;
    use crate::inference_socket::inference_socket_exchange::InferenceSocketExchange;
    use crate::inference_socket::outbound_request::OutboundRequest;

    #[tokio::test]
    async fn a_binary_frame_drops_every_pending_request_without_waiting_for_more_frames() {
        let exchange = Arc::new(InferenceSocketExchange::default());
        let mut response_rx = exchange
            .register(
                OutboundRequest::serialize("request-1".to_owned(), &json!({}))
                    .expect("a JSON request must serialize"),
            )
            .expect("an open connection must accept a request");

        spawn_read_task(
            iter([Ok(WsMessage::Binary(vec![0].into()))]).chain(pending()),
            InboundMessageRouter { exchange },
        )
        .await
        .expect("the read task must not panic");

        assert_eq!(
            response_rx
                .recv()
                .await
                .and_then(Result::err)
                .as_ref()
                .map(ToString::to_string),
            Some(
                Error::ConnectionDropped {
                    request_id: "request-1".to_owned(),
                }
                .to_string()
            )
        );
    }

    #[tokio::test]
    async fn a_ping_between_frames_leaves_the_following_response_for_its_request() {
        let exchange = Arc::new(InferenceSocketExchange::default());
        let mut response_rx = exchange
            .register(
                OutboundRequest::serialize("request-1".to_owned(), &json!({}))
                    .expect("a JSON request must serialize"),
            )
            .expect("an open connection must accept a request");

        spawn_read_task(
            iter([
                Ok(WsMessage::Ping(Bytes::new())),
                Ok(WsMessage::Text(
                    json!({
                        "Response": {
                            "generated_by": null,
                            "request_id": "request-1",
                            "response": { "Embedding": "Done" }
                        }
                    })
                    .to_string()
                    .into(),
                )),
            ]),
            InboundMessageRouter { exchange },
        )
        .await
        .expect("the read task must not panic");

        assert_eq!(
            response_rx.recv().await.and_then(Result::ok),
            Some(InferenceMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: "request-1".to_owned(),
                response: Response::Embedding(EmbeddingResult::Done),
            }))
        );
    }
}
