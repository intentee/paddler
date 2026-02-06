pub mod identity_transformer;
pub mod transforms_outgoing_message;

use async_trait::async_trait;
use tokio::sync::mpsc;

use self::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer::inference_client::Message as OutgoingMessage;
use crate::controls_session::ControlsSession;

#[derive(Clone)]
pub struct ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync,
{
    chunk_tx: mpsc::UnboundedSender<String>,
    transformer: TTransformsOutgoingMessage,
}

impl<TTransformsOutgoingMessage> ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync,
{
    pub fn new(
        chunk_tx: mpsc::UnboundedSender<String>,
        transformer: TTransformsOutgoingMessage,
    ) -> Self {
        Self {
            chunk_tx,
            transformer,
        }
    }
}

#[async_trait]
impl<TTransformsOutgoingMessage> ControlsSession<OutgoingMessage>
    for ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync,
{
    async fn send_response(&mut self, message: OutgoingMessage) -> anyhow::Result<()> {
        let transformed_message = self.transformer.transform(message).await?;
        let stringified_message = self.transformer.stringify(&transformed_message)?;

        self.chunk_tx.send(stringified_message)?;

        Ok(())
    }
}
