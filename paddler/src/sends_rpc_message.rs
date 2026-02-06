use anyhow::Result;
use async_trait::async_trait;
use paddler_types::rpc_message::RpcMessage;

#[async_trait]
pub trait SendsRpcMessage {
    type Message: RpcMessage;

    async fn send_rpc_message(&self, message: Self::Message) -> Result<()>;
}
