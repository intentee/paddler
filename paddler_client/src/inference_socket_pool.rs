use paddler_types::inference_client::Message as InferenceMessage;
use serde::Serialize;
use serde_json::to_string;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use url::Url;

use crate::error::Error;
use crate::error::Result;
use crate::inference_socket_connection::InferenceSocketConnection;

pub struct InferenceSocketPool {
    url: Url,
    connections: Mutex<Vec<Option<InferenceSocketConnection>>>,
    pool_size: usize,
    next_idx: Mutex<usize>,
}

impl InferenceSocketPool {
    pub fn new(url: Url, pool_size: usize) -> Self {
        Self {
            url,
            connections: Mutex::new((0..pool_size).map(|_| None).collect()),
            pool_size,
            next_idx: Mutex::new(0),
        }
    }

    pub async fn send_request<TMessage: Serialize>(
        &self,
        request_id: String,
        message: TMessage,
    ) -> Result<UnboundedReceiver<Result<InferenceMessage>>> {
        let json = to_string(&message)?;
        let conn_idx = self.next_connection_index().await;

        self.ensure_connection(conn_idx).await?;

        let send_result = {
            let connections = self.connections.lock().await;
            let connection = connections[conn_idx].as_ref().ok_or(Error::PoolExhausted)?;

            connection.send(request_id.clone(), json.clone())
        };

        match send_result {
            Ok(response_rx) => Ok(response_rx),
            Err(Error::ConnectionDropped { .. }) => {
                self.ensure_connection(conn_idx).await?;

                let connections = self.connections.lock().await;
                let connection = connections[conn_idx].as_ref().ok_or(Error::PoolExhausted)?;

                connection
                    .send(request_id, json)
                    .map_err(|reconnection_error| Error::Other(reconnection_error.to_string()))
            }
            Err(other_error) => Err(other_error),
        }
    }

    async fn next_connection_index(&self) -> usize {
        let mut idx = self.next_idx.lock().await;
        let conn_idx = *idx % self.pool_size;
        *idx = idx.wrapping_add(1);

        conn_idx
    }

    async fn ensure_connection(&self, index: usize) -> Result<()> {
        let needs_connect = {
            let connections = self.connections.lock().await;
            let slot = &connections[index];

            slot.is_none()
                || slot
                    .as_ref()
                    .is_some_and(|connection| connection.is_disconnected())
        };

        if needs_connect {
            let new_connection = InferenceSocketConnection::connect(self.url.clone()).await?;
            let mut connections = self.connections.lock().await;
            connections[index] = Some(new_connection);
        }

        Ok(())
    }
}
