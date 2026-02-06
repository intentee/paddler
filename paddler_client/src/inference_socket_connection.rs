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
use crate::inference_socket_read_task::spawn_inference_socket_read_task;
use crate::inference_socket_url::inference_socket_url;
use crate::inference_socket_write_task::spawn_inference_socket_write_task;

pub type PendingRequests = Arc<DashMap<String, UnboundedSender<Result<InferenceMessage>>>>;

pub struct InferenceSocketConnection {
    write_tx: UnboundedSender<String>,
    pending: PendingRequests,
    _read_task: JoinHandle<()>,
    _write_task: JoinHandle<()>,
}

impl InferenceSocketConnection {
    pub async fn connect(url: Url) -> Result<Self> {
        let ws_url = inference_socket_url(url)?;
        let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
        let (ws_write, ws_read) = ws_stream.split();

        let pending: PendingRequests = Arc::new(DashMap::new());
        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        let write_task = spawn_inference_socket_write_task(ws_write, write_rx);
        let read_task = spawn_inference_socket_read_task(ws_read, pending.clone());

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
