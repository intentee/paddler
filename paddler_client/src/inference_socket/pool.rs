use std::sync::Arc;

use paddler_types::inference_client::Message as InferenceMessage;
use serde::Serialize;
use serde_json::to_string;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use url::Url;

use crate::error::Error;
use crate::error::Result;
use crate::inference_socket::connection::Connection;

pub struct Pool {
    url: Url,
    connections: Mutex<Vec<Option<Arc<Connection>>>>,
    capacity: usize,
    next_idx: Mutex<usize>,
}

impl Pool {
    pub fn new(url: Url, capacity: usize) -> Self {
        Self {
            url,
            connections: Mutex::new((0..capacity).map(|_| None).collect()),
            capacity,
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

        let connection = self.get_connection(conn_idx).await?;
        let send_result = connection.send(request_id.clone(), json.clone());

        match send_result {
            Ok(response_rx) => Ok(response_rx),
            Err(Error::ConnectionDropped { .. }) => {
                self.ensure_connection(conn_idx).await?;

                let connection = self.get_connection(conn_idx).await?;

                connection
                    .send(request_id, json)
                    .map_err(|reconnection_error| Error::Other(reconnection_error.to_string()))
            }
            Err(other_error) => Err(other_error),
        }
    }

    async fn get_connection(&self, index: usize) -> Result<Arc<Connection>> {
        let connections = self.connections.lock().await;

        connections[index].clone().ok_or(Error::ConnectionSlotEmpty)
    }

    async fn next_connection_index(&self) -> usize {
        let mut idx = self.next_idx.lock().await;
        let conn_idx = *idx % self.capacity;
        *idx = idx.wrapping_add(1);

        conn_idx
    }

    async fn ensure_connection(&self, index: usize) -> Result<()> {
        let needs_connect = {
            let connections = self.connections.lock().await;

            connections[index]
                .as_ref()
                .is_none_or(|connection| connection.is_disconnected())
        };

        if needs_connect {
            let new_connection = Connection::connect(self.url.clone()).await?;
            let mut connections = self.connections.lock().await;
            connections[index] = Some(Arc::new(new_connection));
        }

        Ok(())
    }
}
