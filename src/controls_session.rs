use anyhow::Result;
use async_trait::async_trait;
use log::error;

use crate::rpc_message::RpcMessage;

#[async_trait]
pub trait ControlsSession<TResponse>: Send + Sync
where
    TResponse: RpcMessage + Sync + 'static,
{
    async fn send_response(&mut self, message: TResponse) -> Result<()>;

    async fn send_response_safe(&mut self, message: TResponse) {
        if let Err(err) = self.send_response(message).await {
            error!("Failed to send response: {err}");
        }
    }
}
