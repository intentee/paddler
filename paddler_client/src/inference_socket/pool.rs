use std::num::NonZeroUsize;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;
use url::Url;

use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::error::Error;
use crate::inference_message_stream::InferenceMessageStream;
use crate::inference_socket::connection::Connection;
use crate::inference_socket::connection_slot::ConnectionSlot;
use crate::inference_socket::round_robin_connection_slots::RoundRobinConnectionSlots;
use crate::result::Result;

struct EstablishedRequest {
    connection: Arc<Connection>,
    response_rx: UnboundedReceiver<Result<InferenceMessage>>,
}

pub struct Pool {
    connection_slots: RoundRobinConnectionSlots,
    url: Url,
}

impl Pool {
    #[must_use]
    pub fn new(url: Url, capacity: NonZeroUsize) -> Self {
        Self {
            connection_slots: RoundRobinConnectionSlots::new(capacity),
            url,
        }
    }

    /// # Errors
    ///
    /// Returns [`Error::InferenceRequestCancelled`] when the cancellation token fires before the request reaches an inference socket, [`Error::InferenceSocketUrlSchemeRejected`] when the client URL cannot be turned into an inference socket URL, [`Error::WebSocket`] when the inference socket cannot be opened, [`Error::Json`] when the request cannot be serialized, [`Error::ConnectionDropped`] when the inference socket closed before the request was registered, and [`Error::InferenceRequestIdInFlight`] when a request with the same id is already in flight on the connection.
    pub async fn send_request<TMessage: Serialize + Sync>(
        &self,
        cancellation_token: CancellationToken,
        request_id: String,
        message: TMessage,
    ) -> Result<InferenceMessageStream> {
        let Some(established_request_result) = cancellation_token
            .run_until_cancelled(self.establish_request(&message, request_id.clone()))
            .await
        else {
            return Err(Error::InferenceRequestCancelled { request_id });
        };

        let EstablishedRequest {
            connection,
            response_rx,
        } = established_request_result?;

        Ok(Box::pin(connection.response_stream(
            cancellation_token,
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

        let connection = Arc::new(Connection::connect(self.url.clone()).await?);

        *connection_slot = ConnectionSlot::Connected(connection.clone());
        drop(connection_slot);

        Ok(connection)
    }

    async fn establish_request<TMessage: Serialize + Sync>(
        &self,
        message: &TMessage,
        request_id: String,
    ) -> Result<EstablishedRequest> {
        let connection = self
            .connected(self.connection_slots.next_connection_slot())
            .await?;
        let response_rx = connection.send(request_id, message)?;

        Ok(EstablishedRequest {
            connection,
            response_rx,
        })
    }
}
