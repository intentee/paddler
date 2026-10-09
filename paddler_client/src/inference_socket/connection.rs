use std::sync::Arc;

use futures_util::StreamExt;
use serde_json::to_string;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use url::Url;

use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

use crate::error::Error;
use crate::error::Result;
use crate::inference_socket::cluster_inference_mode_broadcaster::ClusterInferenceModeBroadcaster;
use crate::inference_socket::pending_requests::PendingRequests;
use crate::inference_socket::spawn_read_task::spawn_read_task;
use crate::inference_socket::spawn_write_task::spawn_write_task;
use crate::inference_socket::url::url;

pub struct Connection {
    pending: Arc<PendingRequests>,
    read_task: JoinHandle<()>,
    write_task: JoinHandle<()>,
    write_tx: UnboundedSender<String>,
}

impl Connection {
    pub async fn connect(
        connection_url: Url,
        cluster_inference_mode_broadcaster: Arc<ClusterInferenceModeBroadcaster>,
    ) -> Result<Self> {
        let ws_url = url(connection_url)?;
        let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
        let (ws_write, ws_read) = ws_stream.split();
        let pending = Arc::new(PendingRequests::default());
        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        Ok(Self {
            read_task: spawn_read_task(
                ws_read,
                pending.clone(),
                cluster_inference_mode_broadcaster,
            ),
            write_task: spawn_write_task(ws_write, write_rx, pending.clone()),
            pending,
            write_tx,
        })
    }

    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        self.pending.is_closed()
    }

    pub fn send(
        &self,
        request_id: String,
        json: String,
    ) -> Result<UnboundedReceiver<Result<InferenceMessage>>> {
        let response_rx = self.pending.register(request_id.clone())?;

        if self.write_tx.send(json).is_err() {
            self.pending.remove(&request_id);

            return Err(Error::ConnectionDropped { request_id });
        }

        Ok(response_rx)
    }

    pub fn stop_responding_to(&self, request_id: String) -> Result<()> {
        self.pending.remove(&request_id);

        let stop_responding_to: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Notification(InferenceServerNotification::StopRespondingTo(
                request_id.clone(),
            ));
        let json = to_string(&stop_responding_to)?;

        self.write_tx
            .send(json)
            .map_err(|_closed_channel| Error::ConnectionDropped { request_id })
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.read_task.abort();
        self.write_task.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    use tokio_tungstenite::tungstenite::Error as WebSocketError;
    use url::Url;

    use super::Connection;
    use crate::error::Error;
    use crate::inference_socket::cluster_inference_mode_broadcaster::ClusterInferenceModeBroadcaster;

    #[tokio::test]
    async fn connect_fails_for_an_unreachable_server() {
        assert!(matches!(
            Connection::connect(
                Url::parse("http://127.0.0.1:1").expect("the test URL must be valid"),
                Arc::new(ClusterInferenceModeBroadcaster::new(NonZeroUsize::MIN)),
            )
            .await,
            Err(Error::WebSocket(WebSocketError::Io(io_error)))
                if io_error.kind() == ErrorKind::ConnectionRefused
        ));
    }
}
