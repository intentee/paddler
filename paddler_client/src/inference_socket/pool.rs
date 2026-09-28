use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::notification::Notification;
use serde::Serialize;
use serde_json::to_string;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::error::Error;
use crate::error::Result;
use crate::inference_message_stream::InferenceMessageStream;
use crate::inference_socket::connection::Connection;
use crate::inference_socket::connection_slot::ConnectionSlot;
use crate::inference_socket::response_stream::response_stream;

struct EstablishedRequest {
    connection: Arc<Connection>,
    response_rx: UnboundedReceiver<Result<InferenceMessage>>,
}

pub struct Pool {
    connection_slots: Vec<Mutex<ConnectionSlot>>,
    next_slot_index: AtomicUsize,
    notification_tx: broadcast::Sender<Notification>,
    url: Url,
}

impl Pool {
    #[must_use]
    pub fn new(url: Url, capacity: NonZeroUsize) -> Self {
        let (notification_tx, _initial_notification_rx) = broadcast::channel(capacity.get());

        Self {
            connection_slots: (0..capacity.get())
                .map(|_slot_index| Mutex::new(ConnectionSlot::Empty))
                .collect(),
            next_slot_index: AtomicUsize::new(0),
            notification_tx,
            url,
        }
    }

    pub fn subscribe_to_notifications(&self) -> broadcast::Receiver<Notification> {
        self.notification_tx.subscribe()
    }

    pub async fn send_request<TMessage: Serialize>(
        &self,
        cancellation_token: CancellationToken,
        request_id: String,
        message: TMessage,
    ) -> Result<InferenceMessageStream> {
        let json = to_string(&message)?;

        let Some(established_request_result) = cancellation_token
            .run_until_cancelled(self.establish_request(json, request_id.clone()))
            .await
        else {
            return Err(Error::InferenceRequestCancelled { request_id });
        };

        let EstablishedRequest {
            connection,
            response_rx,
        } = established_request_result?;

        Ok(Box::pin(response_stream(
            cancellation_token,
            connection,
            request_id,
            response_rx,
        )))
    }

    async fn connected(&self, connection_slot: &Mutex<ConnectionSlot>) -> Result<Arc<Connection>> {
        let mut connection_slot = connection_slot.lock().await;

        if let ConnectionSlot::Connected(connection) = &*connection_slot
            && !connection.is_disconnected()
        {
            return Ok(connection.clone());
        }

        let connection =
            Arc::new(Connection::connect(self.url.clone(), self.notification_tx.clone()).await?);

        *connection_slot = ConnectionSlot::Connected(connection.clone());

        Ok(connection)
    }

    async fn establish_request(
        &self,
        json: String,
        request_id: String,
    ) -> Result<EstablishedRequest> {
        let connection = self.connected(self.next_connection_slot()).await?;
        let response_rx = connection.send(request_id, json)?;

        Ok(EstablishedRequest {
            connection,
            response_rx,
        })
    }

    fn next_connection_slot(&self) -> &Mutex<ConnectionSlot> {
        let slot_index = self.next_slot_index.fetch_add(1, Ordering::Relaxed);

        &self.connection_slots[slot_index % self.connection_slots.len()]
    }
}
