use std::sync::Arc;

use log::error;
use log::warn;
use serde_json::from_str;

use paddler_messaging::inference_client::identified_message::IdentifiedMessage;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::streamable_result::StreamableResult;

use crate::error::Error;
use crate::inference_socket::cluster_inference_mode_broadcaster::ClusterInferenceModeBroadcaster;
use crate::inference_socket::pending_requests::PendingRequests;

fn response_is_terminal(response: &Response) -> bool {
    match response {
        Response::Decision(result) => result.is_done(),
        Response::Embedding(result) => result.is_done(),
        Response::GeneratedToken(result) => result.is_done(),
    }
}

struct RequestScopedMessage {
    is_done: bool,
    request_id: String,
}

pub struct InboundMessageRouter {
    pub cluster_inference_mode_broadcaster: Arc<ClusterInferenceModeBroadcaster>,
    pub pending: Arc<PendingRequests>,
}

impl InboundMessageRouter {
    pub fn route_text(&self, text: &str) {
        match from_str::<InferenceMessage>(text) {
            Ok(message) => self.route(message),
            Err(decoding_error) => match from_str::<IdentifiedMessage>(text) {
                Ok(identified_message) => {
                    let request_id = identified_message.into_request_id();

                    error!("Failed to decode a message for request {request_id}: {decoding_error}");

                    if !self.pending.fail(
                        &request_id,
                        Error::UndecodableInferenceMessage {
                            request_id: request_id.clone(),
                            source: decoding_error,
                        },
                    ) {
                        warn!(
                            "Received an undecodable message for unknown request_id: {request_id}"
                        );
                    }
                }
                Err(identification_error) => {
                    error!(
                        "Failed to decode a message that names no request: {decoding_error}; {identification_error}"
                    );
                }
            },
        }
    }

    fn route(&self, message: InferenceMessage) {
        let request_scoped_message = match &message {
            InferenceMessage::Error(envelope) => RequestScopedMessage {
                is_done: true,
                request_id: envelope.request_id.clone(),
            },
            InferenceMessage::Notification(Notification::ClusterInferenceMode(inference_mode)) => {
                self.cluster_inference_mode_broadcaster
                    .publish(*inference_mode);

                return;
            }
            InferenceMessage::Response(envelope) => RequestScopedMessage {
                is_done: response_is_terminal(&envelope.response),
                request_id: envelope.request_id.clone(),
            },
        };

        if !self.pending.deliver(
            &request_scoped_message.request_id,
            message,
            request_scoped_message.is_done,
        ) {
            warn!(
                "Received message for unknown request_id: {}",
                request_scoped_message.request_id
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tokio::sync::mpsc::error::TryRecvError;

    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::InboundMessageRouter;
    use crate::error::Error;
    use crate::inference_socket::cluster_inference_mode_broadcaster::ClusterInferenceModeBroadcaster;
    use crate::inference_socket::pending_requests::PendingRequests;

    fn router_for(pending: &Arc<PendingRequests>) -> InboundMessageRouter {
        InboundMessageRouter {
            cluster_inference_mode_broadcaster: Arc::new(ClusterInferenceModeBroadcaster::default()),
            pending: Arc::clone(pending),
        }
    }

    #[test]
    fn fails_the_request_whose_message_cannot_be_decoded() {
        let pending = Arc::new(PendingRequests::default());
        let mut response_rx = pending
            .register("request-1".to_owned())
            .expect("an open registry must accept a request");

        router_for(&pending).route_text(
            &json!({
                "Response": {
                    "generated_by": null,
                    "request_id": "request-1",
                    "response": { "GeneratedToken": { "NoSuchResult": "piece" } }
                }
            })
            .to_string(),
        );

        assert!(matches!(
            response_rx.try_recv(),
            Ok(Err(Error::UndecodableInferenceMessage { request_id, .. })) if request_id == "request-1"
        ));
    }

    #[test]
    fn leaves_other_requests_pending_when_an_undecodable_message_names_an_unknown_request() {
        let pending = Arc::new(PendingRequests::default());
        let mut response_rx = pending
            .register("request-1".to_owned())
            .expect("an open registry must accept a request");

        router_for(&pending).route_text(
            &json!({ "Error": { "request_id": "request-2", "error": { "NoSuchError": {} } } })
                .to_string(),
        );

        assert!(
            response_rx
                .try_recv()
                .is_err_and(|receive_error| receive_error == TryRecvError::Empty)
        );
    }

    #[test]
    fn leaves_requests_pending_when_an_undecodable_message_names_no_request() {
        let pending = Arc::new(PendingRequests::default());
        let mut response_rx = pending
            .register("request-1".to_owned())
            .expect("an open registry must accept a request");

        router_for(&pending).route_text("not json");

        assert!(
            response_rx
                .try_recv()
                .is_err_and(|receive_error| receive_error == TryRecvError::Empty)
        );
    }

    #[test]
    fn a_finished_embedding_batch_releases_its_request() {
        let pending = Arc::new(PendingRequests::default());
        let mut response_rx = pending
            .register("request-1".to_owned())
            .expect("an open registry must accept a request");

        router_for(&pending).route_text(
            &json!({
                "Response": {
                    "generated_by": null,
                    "request_id": "request-1",
                    "response": { "Embedding": "Done" }
                }
            })
            .to_string(),
        );

        assert!(matches!(
            response_rx.try_recv(),
            Ok(Ok(InferenceMessage::Response(ResponseEnvelope {
                response: Response::Embedding(embedding_result),
                ..
            }))) if embedding_result == EmbeddingResult::Done
        ));
        assert!(
            response_rx
                .try_recv()
                .is_err_and(|receive_error| receive_error == TryRecvError::Disconnected)
        );
    }

    #[test]
    fn a_notification_nobody_listens_to_leaves_requests_pending() {
        let pending = Arc::new(PendingRequests::default());
        let mut response_rx = pending
            .register("request-1".to_owned())
            .expect("an open registry must accept a request");

        router_for(&pending).route_text(
            &json!({ "Notification": { "ClusterInferenceMode": "Embeddings" } }).to_string(),
        );

        assert!(
            response_rx
                .try_recv()
                .is_err_and(|receive_error| receive_error == TryRecvError::Empty)
        );
    }
}
