use anyhow::Result;
use async_trait::async_trait;
use crate::balancer::inference_client::Message as OutgoingMessage;

use super::transform_result::TransformResult;

#[async_trait]
pub trait TransformsOutgoingMessage {
    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>>;
}
