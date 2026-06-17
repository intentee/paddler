use futures_util::Stream;
use futures_util::StreamExt;
use log::debug;
use log::error;
use log::warn;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::streamable_result::StreamableResult;
use serde_json::from_str;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::error::Error;
use crate::inference_socket::pending_requests::PendingRequests;

fn is_terminal_message(message: &InferenceMessage) -> bool {
    match message {
        InferenceMessage::Error(_) => true,
        InferenceMessage::Response(envelope) => match &envelope.response {
            Response::Embedding(result) => result.is_done(),
            Response::GeneratedToken(result) => result.is_done(),
            Response::Timeout | Response::TooManyBufferedRequests => true,
        },
    }
}

pub fn spawn_read_task<TWebSocketStream>(
    ws_read: TWebSocketStream,
    pending: PendingRequests,
) -> JoinHandle<()>
where
    TWebSocketStream: Stream<Item = Result<WsMessage, WsError>> + Send + Unpin + 'static,
{
    tokio::spawn(async move {
        let mut ws_read = ws_read;

        while let Some(msg_result) = ws_read.next().await {
            match msg_result {
                Ok(WsMessage::Text(text)) => match from_str::<InferenceMessage>(&text) {
                    Ok(message) => {
                        let request_id = match &message {
                            InferenceMessage::Error(envelope) => envelope.request_id.clone(),
                            InferenceMessage::Response(envelope) => envelope.request_id.clone(),
                        };

                        if let Some(sender) = pending.get(&request_id) {
                            let is_done = is_terminal_message(&message);
                            let send_failed = sender.send(Ok(message)).is_err();

                            if is_done || send_failed {
                                drop(sender);
                                pending.remove(&request_id);
                            }
                        } else {
                            warn!("Received message for unknown request_id: {request_id}");
                        }
                    }
                    Err(err) => {
                        error!("Failed to deserialize WebSocket message: {err}");
                    }
                },
                Ok(WsMessage::Close(_)) => break,
                Ok(WsMessage::Ping(_) | WsMessage::Pong(_)) => {}
                Ok(WsMessage::Binary(_) | WsMessage::Frame(_)) => {
                    warn!("Received unexpected binary WebSocket message");
                }
                Err(err) => {
                    error!("WebSocket read error: {err}");
                    break;
                }
            }
        }

        for entry in pending.iter() {
            if entry
                .value()
                .send(Err(Error::ConnectionDropped {
                    request_id: entry.key().clone(),
                }))
                .is_err()
            {
                debug!("Receiver already dropped for request: {}", entry.key());
            }
        }

        pending.clear();
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dashmap::DashMap;
    use futures_util::Stream;
    use futures_util::stream;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use serde_json::to_string;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::UnboundedReceiver;
    use tokio_tungstenite::tungstenite::Error as WsError;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    use super::is_terminal_message;
    use super::spawn_read_task;
    use crate::error::Error;
    use crate::inference_socket::pending_requests::PendingRequests;

    type Delivered = std::result::Result<InferenceMessage, Error>;
    type Frame = std::result::Result<WsMessage, WsError>;

    fn error_message(request_id: &str) -> InferenceMessage {
        InferenceMessage::Error(ErrorEnvelope {
            request_id: request_id.to_owned(),
            error: JsonRpcError {
                code: 504,
                description: "timeout".to_owned(),
            },
        })
    }

    fn token_message(request_id: &str, result: GeneratedTokenResult) -> InferenceMessage {
        InferenceMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: request_id.to_owned(),
            response: Response::GeneratedToken(result),
        })
    }

    fn done_message(request_id: &str) -> InferenceMessage {
        token_message(request_id, GeneratedTokenResult::Done(GenerationSummary::default()))
    }

    fn content_token_message(request_id: &str) -> InferenceMessage {
        token_message(request_id, GeneratedTokenResult::ContentToken("hi".to_owned()))
    }

    fn request_id_of(message: &InferenceMessage) -> &str {
        match message {
            InferenceMessage::Error(envelope) => &envelope.request_id,
            InferenceMessage::Response(envelope) => &envelope.request_id,
        }
    }

    fn text(message: &InferenceMessage) -> WsMessage {
        WsMessage::Text(to_string(message).expect("message serializes").into())
    }

    fn frames(items: Vec<Frame>) -> impl Stream<Item = Frame> + Send + Unpin + 'static {
        stream::iter(items)
    }

    fn insert_request(pending: &PendingRequests, request_id: &str) -> UnboundedReceiver<Delivered> {
        let (sender, receiver) = mpsc::unbounded_channel::<Delivered>();

        pending.insert(request_id.to_owned(), sender);

        receiver
    }

    fn enable_logging() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    }

    #[tokio::test]
    async fn keeps_a_non_terminal_response_pending_for_a_live_receiver() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let mut responses = insert_request(&pending, "r1");

        spawn_read_task(
            frames(vec![Ok(text(&content_token_message("r1"))), Ok(text(&error_message("r1")))]),
            pending.clone(),
        )
        .await
        .expect("read task joins");

        let first = responses.recv().await.expect("a message").expect("a success");
        let second = responses.recv().await.expect("a message").expect("a success");

        assert_eq!(request_id_of(&first), "r1");
        assert_eq!(request_id_of(&second), "r1");
    }

    #[test]
    fn an_embedding_response_is_terminal_when_done() {
        let message = InferenceMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "r".to_owned(),
            response: Response::Embedding(EmbeddingResult::Done),
        });

        assert!(is_terminal_message(&message));
    }

    #[test]
    fn a_timeout_response_is_terminal() {
        let message = InferenceMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "r".to_owned(),
            response: Response::Timeout,
        });

        assert!(is_terminal_message(&message));
    }

    #[test]
    fn a_too_many_buffered_requests_response_is_terminal() {
        let message = InferenceMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "r".to_owned(),
            response: Response::TooManyBufferedRequests,
        });

        assert!(is_terminal_message(&message));
    }

    #[tokio::test]
    async fn delivers_a_terminal_error_to_the_waiting_request() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let mut responses = insert_request(&pending, "r1");

        spawn_read_task(frames(vec![Ok(text(&error_message("r1")))]), pending.clone())
            .await
            .expect("read task joins");

        let delivered = responses.recv().await.expect("a message").expect("a success");

        assert_eq!(request_id_of(&delivered), "r1");
    }

    #[tokio::test]
    async fn skips_a_malformed_frame_and_delivers_the_next_response() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let mut responses = insert_request(&pending, "r1");

        spawn_read_task(
            frames(vec![
                Ok(WsMessage::Text("not valid json".into())),
                Ok(text(&done_message("r1"))),
            ]),
            pending.clone(),
        )
        .await
        .expect("read task joins");

        let delivered = responses.recv().await.expect("a message").expect("a success");

        assert_eq!(request_id_of(&delivered), "r1");
    }

    #[tokio::test]
    async fn ignores_a_response_for_an_unknown_request_and_delivers_the_known_one() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let mut responses = insert_request(&pending, "known");

        spawn_read_task(
            frames(vec![
                Ok(text(&error_message("unknown"))),
                Ok(text(&error_message("known"))),
            ]),
            pending.clone(),
        )
        .await
        .expect("read task joins");

        let delivered = responses.recv().await.expect("a message").expect("a success");

        assert_eq!(request_id_of(&delivered), "known");
    }

    #[tokio::test]
    async fn ignores_control_and_binary_frames_then_delivers_a_response() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let mut responses = insert_request(&pending, "r1");

        spawn_read_task(
            frames(vec![
                Ok(WsMessage::Ping(Vec::<u8>::new().into())),
                Ok(WsMessage::Pong(Vec::<u8>::new().into())),
                Ok(WsMessage::Binary(Vec::<u8>::new().into())),
                Ok(text(&error_message("r1"))),
            ]),
            pending.clone(),
        )
        .await
        .expect("read task joins");

        let delivered = responses.recv().await.expect("a message").expect("a success");

        assert_eq!(request_id_of(&delivered), "r1");
    }

    #[tokio::test]
    async fn fails_pending_requests_when_the_socket_sends_a_close_frame() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let mut responses = insert_request(&pending, "r1");

        spawn_read_task(
            frames(vec![Ok(WsMessage::Close(None)), Ok(text(&error_message("r1")))]),
            pending.clone(),
        )
        .await
        .expect("read task joins");

        assert!(responses.recv().await.expect("a message").is_err());
    }

    #[tokio::test]
    async fn fails_pending_requests_on_a_socket_read_error() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let mut responses = insert_request(&pending, "r1");

        spawn_read_task(
            frames(vec![Err(WsError::ConnectionClosed), Ok(text(&error_message("r1")))]),
            pending.clone(),
        )
        .await
        .expect("read task joins");

        assert!(responses.recv().await.expect("a message").is_err());
    }

    #[tokio::test]
    async fn stops_tracking_a_request_whose_receiver_was_dropped_then_keeps_serving_others() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let abandoned = insert_request(&pending, "gone");
        let mut live = insert_request(&pending, "live");

        drop(abandoned);

        spawn_read_task(
            frames(vec![
                Ok(text(&content_token_message("gone"))),
                Ok(text(&error_message("live"))),
            ]),
            pending.clone(),
        )
        .await
        .expect("read task joins");

        let delivered = live.recv().await.expect("a message").expect("a success");

        assert_eq!(request_id_of(&delivered), "live");
    }

    #[tokio::test]
    async fn tolerates_an_already_dropped_receiver_when_draining_on_disconnect() {
        let pending: PendingRequests = Arc::new(DashMap::new());
        enable_logging();
        let abandoned = insert_request(&pending, "gone");

        drop(abandoned);

        spawn_read_task(frames(Vec::new()), pending.clone())
            .await
            .expect("read task joins");

        assert!(pending.is_empty());
    }
}
