use std::sync::Arc;

use dashmap::DashMap;
use futures_util::StreamExt;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
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
    _read_task: JoinHandle<()>,
    _write_task: JoinHandle<()>,
}

impl Connection {
    pub async fn connect(connection_url: Url) -> Result<Self> {
        let ws_url = url(connection_url)?;
        let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
        let (ws_write, ws_read) = ws_stream.split();

        let pending: PendingRequests = Arc::new(DashMap::new());
        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        let write_task = spawn_write_task(ws_write, write_rx);
        let read_task = spawn_read_task(ws_read, pending.clone());

        Ok(Self {
            write_tx,
            pending,
            _read_task: read_task,
            _write_task: write_task,
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
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dashmap::DashMap;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::UnboundedSender;
    use url::Url;

    use super::Connection;

    impl Connection {
        fn from_write_channel(write_tx: UnboundedSender<String>) -> Self {
            Self {
                write_tx,
                pending: Arc::new(DashMap::new()),
                _read_task: tokio::spawn(std::future::ready(())),
                _write_task: tokio::spawn(std::future::ready(())),
            }
        }
    }

    #[tokio::test]
    async fn connect_fails_for_an_unreachable_server() {
        let url = Url::parse("http://127.0.0.1:1").unwrap();

        assert!(Connection::connect(url).await.is_err());
    }

    #[tokio::test]
    async fn connect_rejects_an_unsupported_url_scheme() {
        let url = Url::parse("ftp://host/model").unwrap();

        assert!(Connection::connect(url).await.is_err());
    }

    #[tokio::test]
    async fn send_queues_the_request_and_returns_a_receiver() {
        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<String>();
        let connection = Connection::from_write_channel(write_tx);

        let _receiver = connection
            .send("r1".to_owned(), "{}".to_owned())
            .expect("send succeeds for a live connection");

        assert!(!connection.is_disconnected());
        assert_eq!(write_rx.recv().await, Some("{}".to_owned()));
    }

    #[tokio::test]
    async fn send_reports_connection_dropped_when_the_write_channel_is_closed() {
        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        drop(write_rx);

        let connection = Connection::from_write_channel(write_tx);

        assert!(connection.is_disconnected());
        assert!(connection.send("r1".to_owned(), "{}".to_owned()).is_err());
    }
}
