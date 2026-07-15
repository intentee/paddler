use std::sync::Arc;

use futures_util::Stream;
use futures_util::stream::unfold;
use log::debug;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::inference_socket::connection::Connection;

struct StreamState {
    cancellation_token: CancellationToken,
    connection: Arc<Connection>,
    request_id: String,
    response_rx: UnboundedReceiver<Result<InferenceMessage>>,
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
            request_id,
            response_rx,
        },
        |mut state| async move {
            let received_message = state
                .cancellation_token
                .run_until_cancelled(state.response_rx.recv())
                .await;

            match received_message {
                None => {
                    if let Err(stop_error) = state
                        .connection
                        .stop_responding_to(state.request_id.clone())
                    {
                        debug!(
                            "Could not ask the balancer to stop responding to request {}: {stop_error}",
                            state.request_id
                        );
                    }

                    None
                }
                Some(None) => None,
                Some(Some(message)) => Some((message, state)),
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::StreamExt as _;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::notification::Notification;
    use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
    use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
    use serde_json::from_str;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::response_stream;
    use crate::inference_socket::connection::Connection;

    #[tokio::test]
    async fn cancelling_a_request_whose_connection_has_closed_ends_the_stream() {
        let (write_tx, write_rx) = mpsc::unbounded_channel::<String>();

        drop(write_rx);

        let (_response_tx, response_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        let mut stream = Box::pin(response_stream(
            cancellation_token,
            Arc::new(Connection::from_write_sender(write_tx)),
            "cancelled_request".to_owned(),
            response_rx,
        ));

        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn cancelling_a_live_request_asks_the_balancer_to_stop_responding() {
        let (write_tx, mut write_rx) = mpsc::unbounded_channel::<String>();
        let (_response_tx, response_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        let mut stream = Box::pin(response_stream(
            cancellation_token,
            Arc::new(Connection::from_write_sender(write_tx)),
            "live_request".to_owned(),
            response_rx,
        ));

        assert!(stream.next().await.is_none());

        let sent_json = write_rx
            .recv()
            .await
            .expect("cancelling a live request must send a stop message");

        match from_str::<InferenceServerMessage<ValidatedParametersSchema>>(&sent_json)
            .expect("the stop message must be valid JSON-RPC")
        {
            InferenceServerMessage::Notification(
                InferenceServerNotification::StopRespondingTo(request_id),
            ) => assert_eq!(request_id, "live_request"),
            _other_message => panic!("cancelling a request must send a stop notification"),
        }
    }

    #[tokio::test]
    async fn forwards_messages_until_the_response_channel_closes() {
        let (write_tx, _write_rx) = mpsc::unbounded_channel::<String>();
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        response_tx
            .send(Ok(InferenceMessage::Notification(
                Notification::TokenGenerationEnabled,
            )))
            .expect("the response channel must accept the message");

        drop(response_tx);

        let mut stream = Box::pin(response_stream(
            CancellationToken::new(),
            Arc::new(Connection::from_write_sender(write_tx)),
            "streamed_request".to_owned(),
            response_rx,
        ));

        assert!(matches!(
            stream.next().await,
            Some(Ok(InferenceMessage::Notification(
                Notification::TokenGenerationEnabled
            )))
        ));
        assert!(stream.next().await.is_none());
    }
}
