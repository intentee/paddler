use futures_util::StreamExt;
use futures_util::stream::SplitStream;
use log::debug;
use log::error;
use log::warn;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::streamable_result::StreamableResult;
use serde_json::from_str;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::error::Error;
use crate::inference_socket::pending_requests::PendingRequests;

type WebSocketReadStream = SplitStream<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>;

fn response_is_terminal(response: &Response) -> bool {
    match response {
        Response::Embedding(result) => result.is_done(),
        Response::GeneratedToken(result) => result.is_done(),
        Response::Timeout | Response::TooManyBufferedRequests => true,
    }
}

fn route_message(
    pending: &PendingRequests,
    notification_tx: &broadcast::Sender<Notification>,
    message: InferenceMessage,
) {
    let request_scoped_message = match &message {
        InferenceMessage::Error(envelope) => RequestScopedMessage {
            is_done: true,
            request_id: envelope.request_id.clone(),
        },
        InferenceMessage::Notification(notification) => {
            if notification_tx.send(notification.clone()).is_err() {
                debug!("Dropped inference notification: no active subscribers");
            }

            return;
        }
        InferenceMessage::Response(envelope) => RequestScopedMessage {
            is_done: response_is_terminal(&envelope.response),
            request_id: envelope.request_id.clone(),
        },
    };

    if let Some(sender) = pending.get(&request_scoped_message.request_id) {
        let send_failed = sender.send(Ok(message)).is_err();

        if request_scoped_message.is_done || send_failed {
            drop(sender);
            pending.remove(&request_scoped_message.request_id);
        }
    } else {
        warn!(
            "Received message for unknown request_id: {}",
            request_scoped_message.request_id
        );
    }
}

struct RequestScopedMessage {
    is_done: bool,
    request_id: String,
}

#[must_use]
pub fn spawn_read_task(
    ws_read: WebSocketReadStream,
    pending: PendingRequests,
    notification_tx: broadcast::Sender<Notification>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ws_read = ws_read;

        while let Some(msg_result) = ws_read.next().await {
            match msg_result {
                Ok(WsMessage::Text(text)) => match from_str::<InferenceMessage>(&text) {
                    Ok(message) => route_message(&pending, &notification_tx, message),
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
