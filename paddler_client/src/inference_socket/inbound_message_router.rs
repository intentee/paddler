use std::sync::Arc;

use log::error;
use log::warn;
use serde_json::from_str;

use paddler_messaging::inference_client::identified_message::IdentifiedMessage;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::result_step::ResultStep;

use crate::error::Error;
use crate::inference_socket::inference_socket_exchange::InferenceSocketExchange;

struct RequestScopedMessage {
    is_done: bool,
    message: InferenceMessage,
    request_id: String,
}

impl RequestScopedMessage {
    fn of_response(is_done: bool, envelope: ResponseEnvelope<Response>) -> Self {
        Self {
            is_done,
            request_id: envelope.request_id.clone(),
            message: InferenceMessage::Response(envelope),
        }
    }
}

pub struct InboundMessageRouter {
    pub exchange: Arc<InferenceSocketExchange>,
}

impl InboundMessageRouter {
    pub fn route_text(&self, text: &str) {
        match from_str::<InferenceMessage>(text) {
            Ok(message) => self.route(message),
            Err(decoding_error) => match from_str::<IdentifiedMessage>(text) {
                Ok(identified_message) => {
                    let request_id = identified_message.into_request_id();

                    error!("Failed to decode a message for request {request_id}: {decoding_error}");

                    if !self.exchange.fail(
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
        let request_scoped_message = match message {
            InferenceMessage::Error(envelope) => RequestScopedMessage {
                is_done: true,
                request_id: envelope.request_id.clone(),
                message: InferenceMessage::Error(envelope),
            },
            InferenceMessage::Response(ResponseEnvelope {
                generated_by,
                request_id,
                response,
            }) => match response.into_step() {
                ResultStep::Final(final_response) => RequestScopedMessage::of_response(
                    true,
                    ResponseEnvelope {
                        generated_by,
                        request_id,
                        response: final_response,
                    },
                ),
                ResultStep::Partial(partial_response) => RequestScopedMessage::of_response(
                    false,
                    ResponseEnvelope {
                        generated_by,
                        request_id,
                        response: partial_response,
                    },
                ),
            },
        };

        if !self.exchange.deliver(
            &request_scoped_message.request_id,
            request_scoped_message.message,
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

    use paddler_messaging::decision_result::DecisionResult;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::InboundMessageRouter;
    use crate::error::Error;
    use crate::inference_socket::inference_socket_exchange::InferenceSocketExchange;
    use crate::inference_socket::outbound_request::OutboundRequest;

    fn router_for(exchange: &Arc<InferenceSocketExchange>) -> InboundMessageRouter {
        InboundMessageRouter {
            exchange: Arc::clone(exchange),
        }
    }

    #[test]
    fn fails_the_request_whose_message_cannot_be_decoded() {
        let exchange = Arc::new(InferenceSocketExchange::default());
        let mut response_rx = exchange
            .register(
                OutboundRequest::serialize("request-1".to_owned(), &json!({}))
                    .expect("a JSON request must serialize"),
            )
            .expect("an open exchange must accept a request");

        router_for(&exchange).route_text(
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
        let exchange = Arc::new(InferenceSocketExchange::default());
        let mut response_rx = exchange
            .register(
                OutboundRequest::serialize("request-1".to_owned(), &json!({}))
                    .expect("a JSON request must serialize"),
            )
            .expect("an open exchange must accept a request");

        router_for(&exchange).route_text(
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
        let exchange = Arc::new(InferenceSocketExchange::default());
        let mut response_rx = exchange
            .register(
                OutboundRequest::serialize("request-1".to_owned(), &json!({}))
                    .expect("a JSON request must serialize"),
            )
            .expect("an open exchange must accept a request");

        router_for(&exchange).route_text("not json");

        assert!(
            response_rx
                .try_recv()
                .is_err_and(|receive_error| receive_error == TryRecvError::Empty)
        );
    }

    #[test]
    fn a_terminal_response_releases_its_request() {
        for (wire_response, terminal_response) in [
            (
                json!({ "Embedding": "Done" }),
                Response::Embedding(EmbeddingResult::Done),
            ),
            (
                json!({ "Decision": "StopRequested" }),
                Response::Decision(DecisionResult::StopRequested),
            ),
        ] {
            let exchange = Arc::new(InferenceSocketExchange::default());
            let mut response_rx = exchange
                .register(
                    OutboundRequest::serialize("request-1".to_owned(), &json!({}))
                        .expect("a JSON request must serialize"),
                )
                .expect("an open exchange must accept a request");

            router_for(&exchange).route_text(
                &json!({
                    "Response": {
                        "generated_by": null,
                        "request_id": "request-1",
                        "response": wire_response
                    }
                })
                .to_string(),
            );

            assert_eq!(
                response_rx.try_recv().ok().and_then(Result::ok),
                Some(InferenceMessage::Response(ResponseEnvelope {
                    generated_by: None,
                    request_id: "request-1".to_owned(),
                    response: terminal_response,
                }))
            );
            assert!(
                response_rx
                    .try_recv()
                    .is_err_and(|receive_error| receive_error == TryRecvError::Disconnected)
            );
        }
    }
}
