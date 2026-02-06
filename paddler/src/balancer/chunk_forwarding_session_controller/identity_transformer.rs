use anyhow::Result;
use async_trait::async_trait;

use super::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer::inference_client::Message as OutgoingMessage;

#[derive(Clone)]
pub struct IdentityTransformer;

impl IdentityTransformer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TransformsOutgoingMessage for IdentityTransformer {
    type TransformedMessage = OutgoingMessage;

    async fn transform(&self, message: OutgoingMessage) -> Result<Self::TransformedMessage> {
        Ok(message)
    }
}
