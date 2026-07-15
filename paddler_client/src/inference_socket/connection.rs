use std::sync::Arc;

use dashmap::DashMap;
use futures_util::StreamExt;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serde_json::to_string;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use url::Url;

use crate::error::Error;
use crate::error::Result;
use crate::inference_socket::pending_requests::PendingRequests;
use crate::inference_socket::spawn_read_task::spawn_read_task;
use crate::inference_socket::spawn_write_task::spawn_write_task;
use crate::inference_socket::url::url;

pub struct Connection {
    write_tx: UnboundedSender<String>,
    pending: PendingRequests,
    read_task: JoinHandle<()>,
    write_task: JoinHandle<()>,
}

impl Connection {
    pub async fn connect(
        connection_url: Url,
        notification_tx: broadcast::Sender<Notification>,
    ) -> Result<Self> {
        let ws_url = url(connection_url)?;
        let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
        let (ws_write, ws_read) = ws_stream.split();

        let pending: PendingRequests = Arc::new(DashMap::new());
        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        let write_task = spawn_write_task(ws_write, write_rx);
        let read_task = spawn_read_task(ws_read, pending.clone(), notification_tx);

        Ok(Self {
            write_tx,
            pending,
            read_task,
            write_task,
        })
    }

    pub fn is_disconnected(&self) -> bool {
        self.write_tx.is_closed()
    }

    pub fn send(
        &self,
        request_id: String,
        json: String,
    ) -> Result<UnboundedReceiver<Result<InferenceMessage>>> {
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        self.pending.insert(request_id.clone(), response_tx);

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

    #[cfg(test)]
    pub fn from_write_sender(write_tx: UnboundedSender<String>) -> Self {
        Self {
            write_tx,
            pending: Arc::new(DashMap::new()),
            read_task: tokio::spawn(async {}),
            write_task: tokio::spawn(async {}),
        }
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
    use std::time::Duration;

    use futures_util::StreamExt as _;
    use tokio::net::TcpListener;
    use tokio::sync::broadcast;
    use tokio::time::timeout;
    use tokio_tungstenite::accept_async;
    use url::Url;

    use super::Connection;

    #[tokio::test]
    async fn connect_fails_for_an_unreachable_server() {
        let url = Url::parse("http://127.0.0.1:1").unwrap();
        let (notification_tx, _notification_rx) = broadcast::channel(1);

        assert!(Connection::connect(url, notification_tx).await.is_err());
    }

    #[tokio::test]
    async fn dropping_the_connection_closes_the_websocket() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the fixture server must bind a loopback port");
        let address = listener
            .local_addr()
            .expect("the fixture server must report its address");

        let server_task = tokio::spawn(async move {
            let (tcp_stream, _peer_address) = listener
                .accept()
                .await
                .expect("the fixture server must accept the connection");
            let mut server_websocket = accept_async(tcp_stream)
                .await
                .expect("the fixture server must complete the websocket handshake");

            while server_websocket.next().await.is_some() {}
        });

        let url = Url::parse(&format!("http://{address}")).expect("the fixture URL must be valid");
        let (notification_tx, _notification_rx) = broadcast::channel(1);
        let connection = Connection::connect(url, notification_tx)
            .await
            .expect("the client must connect to the fixture server");

        drop(connection);

        timeout(Duration::from_secs(5), server_task)
            .await
            .expect("dropping the connection must close the websocket before the deadline")
            .expect("the fixture server task must not panic");
    }
}
