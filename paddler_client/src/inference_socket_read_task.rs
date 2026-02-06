use futures_util::StreamExt;
use futures_util::stream::SplitStream;
use log::error;
use log::warn;
use paddler_types::inference_client::Message as InferenceMessage;
use paddler_types::inference_client::Response;
use paddler_types::streamable_result::StreamableResult;
use serde_json::from_str;
use tokio::task::JoinHandle;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::error::Error;
use crate::inference_socket_connection::PendingRequests;

type WebSocketReadStream = SplitStream<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>;

fn is_terminal_message(message: &InferenceMessage) -> bool {
    match message {
        InferenceMessage::Error(_) => true,
        InferenceMessage::Response(envelope) => match &envelope.response {
            Response::Embedding(result) => result.is_done(),
            Response::GeneratedToken(result) => result.is_done(),
            Response::Timeout => true,
            Response::TooManyBufferedRequests => true,
        },
    }
}

pub fn spawn_inference_socket_read_task(
    ws_read: WebSocketReadStream,
    pending: PendingRequests,
) -> JoinHandle<()> {
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
                Ok(WsMessage::Ping(_)) | Ok(WsMessage::Pong(_)) => {}
                Ok(WsMessage::Binary(_)) | Ok(WsMessage::Frame(_)) => {
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
                log::debug!("Receiver already dropped for request: {}", entry.key(),);
            }
        }

        pending.clear();
    })
}
