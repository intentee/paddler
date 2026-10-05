use std::future::Future;

use anyhow::Result;
use log::error;

use paddler_messaging::rpc_message::RpcMessage;

pub trait ControlsSession<TResponse>: Send + Sync
where
    TResponse: RpcMessage + Sync + 'static,
{
    fn send_response(&mut self, message: TResponse) -> impl Future<Output = Result<()>> + Send;

    fn send_response_safe(&mut self, message: TResponse) -> impl Future<Output = ()> + Send {
        async move {
            if let Err(err) = self.send_response(message).await {
                error!("Failed to send response: {err}");
            }
        }
    }
}
