use anyhow::Result;
use async_trait::async_trait;
use paddler_types::inference_client::Message as OutgoingMessage;

use super::transform_result::TransformResult;
use super::transforms_outgoing_message::TransformsOutgoingMessage;

#[derive(Clone)]
pub struct IdentityTransformer;

impl IdentityTransformer {
    pub const fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TransformsOutgoingMessage for IdentityTransformer {
    async fn transform(&self, message: OutgoingMessage) -> Result<TransformResult> {
        let serialized = serde_json::to_string(&message)?;

        Ok(TransformResult::Chunk(serialized))
    }
}
