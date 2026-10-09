use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;
use tokio::sync::mpsc;

use paddler_messaging::management_socket::agent::response::Response as JsonRpcResponse;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use paddler_request_registry::request_registry_guard::RequestRegistryGuard;

use crate::agent_response_message::agent_response_message;

pub struct PipelineResponseStream<TResponse> {
    pub response_rx: mpsc::UnboundedReceiver<TResponse>,
    pub stopper_guard: RequestRegistryGuard<mpsc::UnboundedSender<()>>,
}

impl<TResponse> Stream for PipelineResponseStream<TResponse>
where
    TResponse: Into<JsonRpcResponse>,
{
    type Item = ManagementJsonRpcMessage;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Self {
            response_rx,
            stopper_guard,
        } = self.get_mut();

        response_rx.poll_recv(context).map(|received_response| {
            received_response.map(|response| {
                agent_response_message(stopper_guard.request_id.clone(), response.into())
            })
        })
    }
}
