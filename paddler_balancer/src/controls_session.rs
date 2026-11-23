use std::future::Future;

use log::error;

use paddler_error_chain::error_chain::ErrorChain;
use paddler_messaging::rpc_message::RpcMessage;

use crate::balancer_error::BalancerError;

pub trait ControlsSession<TResponse>: Send + Sync
where
    TResponse: RpcMessage + Sync + 'static,
{
    fn send_response(
        &mut self,
        message: TResponse,
    ) -> impl Future<Output = Result<(), BalancerError>> + Send;

    fn send_response_safe(&mut self, message: TResponse) -> impl Future<Output = ()> + Send {
        async move {
            if let Err(send_error) = self.send_response(message).await {
                error!(
                    "Failed to send response: {}",
                    ErrorChain { error: &send_error }
                );
            }
        }
    }
}
