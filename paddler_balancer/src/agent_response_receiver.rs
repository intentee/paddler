use std::sync::Arc;

use tokio::sync::mpsc;

use paddler_request_registry::request_registry_guard::RequestRegistryGuard;

use crate::agent_relay_error::AgentRelayError;
use crate::response_senders::ResponseSenders;

pub struct AgentResponseReceiver<TResponse> {
    pub request_registration: RequestRegistryGuard<mpsc::UnboundedSender<TResponse>>,
    pub response_rx: mpsc::UnboundedReceiver<TResponse>,
}

impl<TResponse> AgentResponseReceiver<TResponse> {
    pub fn register(
        response_senders: &Arc<ResponseSenders<TResponse>>,
        request_id: String,
    ) -> Result<Self, AgentRelayError> {
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        RequestRegistryGuard::register(response_senders, request_id.clone(), response_tx)
            .map(|request_registration| Self {
                request_registration,
                response_rx,
            })
            .ok_or(AgentRelayError::RequestIdAlreadyRelayed { request_id })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_request_registry::request_delivery::RequestDelivery;

    use super::AgentResponseReceiver;
    use crate::agent_relay_error::AgentRelayError;
    use crate::response_senders::ResponseSenders;

    #[test]
    fn receives_the_responses_sent_to_its_request() {
        let response_senders = Arc::new(ResponseSenders::default());
        let mut agent_response_receiver =
            AgentResponseReceiver::register(&response_senders, "request-1".to_owned())
                .expect("a fresh request id must register");

        assert_eq!(
            response_senders.send_to("request-1", EmbeddingResult::Done),
            RequestDelivery::Delivered
        );
        assert_eq!(
            agent_response_receiver.response_rx.try_recv(),
            Ok(EmbeddingResult::Done)
        );
    }

    #[test]
    fn refuses_a_second_receiver_for_the_same_request() {
        let response_senders = Arc::new(ResponseSenders::<EmbeddingResult>::default());
        let _first_receiver =
            AgentResponseReceiver::register(&response_senders, "request-1".to_owned())
                .expect("a fresh request id must register");

        assert!(matches!(
            AgentResponseReceiver::register(&response_senders, "request-1".to_owned()),
            Err(AgentRelayError::RequestIdAlreadyRelayed { request_id }) if request_id == "request-1"
        ));
    }
}
