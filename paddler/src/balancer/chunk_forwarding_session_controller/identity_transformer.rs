use crate::balancer::inference_client::Message as OutgoingMessage;
use anyhow::Result;
use async_trait::async_trait;

use super::transform_result::TransformResult;
use super::transforms_outgoing_message::TransformsOutgoingMessage;

#[derive(Clone, Default)]
pub struct IdentityTransformer;

impl IdentityTransformer {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TransformsOutgoingMessage for IdentityTransformer {
    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        let serialized = serde_json::to_string(&message)?;

        Ok(vec![TransformResult::Chunk(serialized)])
    }
}
