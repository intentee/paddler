use std::sync::Arc;

use paddler_messaging::inference_client::message::Message as InferenceMessage;
use serde::Serialize;
use serde_json::to_string;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use url::Url;

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

        self.connection(conn_idx).await?.send(request_id, json)
    }

    async fn next_connection_index(&self) -> usize {
        let mut idx = self.next_idx.lock().await;
        let conn_idx = *idx % self.capacity;
        *idx = idx.wrapping_add(1);

        conn_idx
    }

    async fn connection(&self, index: usize) -> Result<Arc<Connection>> {
        {
            let connections = self.connections.lock().await;

            if let Some(connection) = &connections[index]
                && !connection.is_disconnected()
            {
                return Ok(connection.clone());
            }
        }

        let connection = Arc::new(Connection::connect(self.url.clone()).await?);
        let mut connections = self.connections.lock().await;

        connections[index] = Some(connection.clone());

        Ok(connection)
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde::Serializer;
    use serde::ser::Error as _;

    use super::Pool;

    struct UnserializableMessage;

    impl Serialize for UnserializableMessage {
        fn serialize<TSerializer>(
            &self,
            _serializer: TSerializer,
        ) -> Result<TSerializer::Ok, TSerializer::Error>
        where
            TSerializer: Serializer,
        {
            Err(TSerializer::Error::custom("this message never serializes"))
        }
    }

    #[tokio::test]
    async fn round_robins_across_connection_slots() {
        let pool = Pool::new(url::Url::parse("http://127.0.0.1:1").unwrap(), 3);

        assert_eq!(pool.next_connection_index().await, 0);
        assert_eq!(pool.next_connection_index().await, 1);
        assert_eq!(pool.next_connection_index().await, 2);
        assert_eq!(pool.next_connection_index().await, 0);
    }

    #[tokio::test]
    async fn send_request_errors_when_the_message_cannot_be_serialized() {
        let pool = Pool::new(url::Url::parse("http://127.0.0.1:1").unwrap(), 1);

        assert!(pool.send_request("r1".to_owned(), UnserializableMessage).await.is_err());
    }
}
