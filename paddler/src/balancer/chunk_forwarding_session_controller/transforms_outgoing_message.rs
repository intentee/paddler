use anyhow::Result;
use async_trait::async_trait;
use paddler_types::inference_client::Message as OutgoingMessage;
use serde::Serialize;

#[async_trait]
pub trait TransformsOutgoingMessage {
    type TransformedMessage: Serialize;

    async fn transform(&self, message: OutgoingMessage) -> Result<Self::TransformedMessage>;

    fn stringify(&self, message: &Self::TransformedMessage) -> Result<String> {
        Ok(serde_json::to_string(message)?)
    }
}
