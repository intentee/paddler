use async_trait::async_trait;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;

use crate::agent_relay_error::AgentRelayError;

#[async_trait]
pub trait TransformsOutgoingMessage {
    type Output: Send + Sync + 'static;

    async fn transform(
        &self,
        message: OutgoingMessage,
    ) -> Result<Vec<Self::Output>, AgentRelayError>;
}
