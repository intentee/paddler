use anyhow::Result;
use async_trait::async_trait;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;

#[async_trait]
pub trait TransformsOutgoingMessage {
    type Output: Send + Sync + 'static;

    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<Self::Output>>;
}
