use std::sync::Arc;

use tokio::sync::mpsc;

use paddler_request_registry::request_registry_guard::RequestRegistryGuard;

use crate::response_senders::ResponseSenders;

pub struct AgentResponseReceiver<TResponse> {
    pub request_registration: RequestRegistryGuard<mpsc::UnboundedSender<TResponse>>,
    pub response_rx: mpsc::UnboundedReceiver<TResponse>,
}

impl<TResponse> AgentResponseReceiver<TResponse> {
    #[must_use]
    pub fn register(response_senders: &Arc<ResponseSenders<TResponse>>) -> Self {
        let (response_tx, response_rx) = mpsc::unbounded_channel();

        Self {
            request_registration: RequestRegistryGuard::register_fresh(
                response_senders,
                response_tx,
            ),
            response_rx,
        }
    }
}
