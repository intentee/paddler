use std::sync::Arc;

use dashmap::DashMap;
use futures_util::StreamExt;
use paddler_types::inference_client::Message as InferenceMessage;
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
