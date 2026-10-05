use tokio::sync::mpsc;

use paddler_request_registry::request_registry::RequestRegistry;

pub type ResponseSenders<TResponse> = RequestRegistry<mpsc::UnboundedSender<TResponse>>;
