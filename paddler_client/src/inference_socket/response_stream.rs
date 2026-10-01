use std::sync::Arc;

use futures_util::Stream;
use futures_util::stream::unfold;
use log::debug;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::error::Result;
use crate::inference_socket::connection::Connection;

struct StreamState {
    cancellation_token: CancellationToken,
    connection: Arc<Connection>,
    is_complete: bool,
    request_id: String,
    response_rx: UnboundedReceiver<Result<InferenceMessage>>,
}

impl StreamState {
    const fn mark_complete(&mut self) {
        self.is_complete = true;
    }
}

impl Drop for StreamState {
    fn drop(&mut self) {
        if self.is_complete {
            return;
        }

        if let Err(stop_error) = self.connection.stop_responding_to(self.request_id.clone()) {
            debug!(
                "Could not ask the balancer to stop responding to request {}: {stop_error}",
                self.request_id
            );
        }
    }
}

pub fn response_stream(
    cancellation_token: CancellationToken,
    connection: Arc<Connection>,
    request_id: String,
    response_rx: UnboundedReceiver<Result<InferenceMessage>>,
) -> impl Stream<Item = Result<InferenceMessage>> + Send + 'static {
    unfold(
        StreamState {
            cancellation_token,
            connection,
            is_complete: false,
            request_id,
            response_rx,
        },
        |mut state| async move {
            let received_message = state
                .cancellation_token
                .run_until_cancelled(state.response_rx.recv())
                .await;

            match received_message {
                None => None,
                Some(None) => {
                    state.mark_complete();

                    None
                }
                Some(Some(message)) => Some((message, state)),
            }
        },
    )
}
