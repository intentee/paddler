use async_trait::async_trait;
use serde_json::to_string;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;

use super::transform_result::TransformResult;
use super::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::agent_relay_error::AgentRelayError;

#[derive(Default)]
pub struct IdentityTransformer;

impl IdentityTransformer {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TransformsOutgoingMessage for IdentityTransformer {
    type Output = TransformResult;

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> Result<Vec<TransformResult>, AgentRelayError> {
        to_string(&message)
            .map(|serialized| vec![TransformResult::Chunk(serialized)])
            .map_err(AgentRelayError::MessageUnserializable)
    }
}
