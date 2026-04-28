pub mod identity_transformer;
pub mod transform_result;
pub mod transforms_outgoing_message;

use async_trait::async_trait;
use paddler_types::inference_client::Message as OutgoingMessage;
use tokio::sync::mpsc;

use self::transform_result::TransformResult;
use self::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::controls_session::ControlsSession;

#[derive(Clone)]
pub struct ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync,
{
    chunk_tx: mpsc::UnboundedSender<TransformResult>,
    transformer: TTransformsOutgoingMessage,
}

impl<TTransformsOutgoingMessage> ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync,
{
    pub const fn new(
        chunk_tx: mpsc::UnboundedSender<TransformResult>,
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
        match self.transformer.transform(message).await? {
            TransformResult::Discard => Ok(()),
            forwarded @ (TransformResult::Chunk(_) | TransformResult::Error(_)) => {
                self.chunk_tx.send(forwarded)?;
                Ok(())
            }
        }
    }
}
