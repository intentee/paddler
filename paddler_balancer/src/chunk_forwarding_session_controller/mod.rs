pub mod identity_transformer;
pub mod transform_result;
pub mod transforms_outgoing_message;

use async_trait::async_trait;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use tokio::sync::mpsc;

use self::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::controls_session::ControlsSession;

#[derive(Clone)]
pub struct ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync,
{
    chunk_tx: mpsc::UnboundedSender<TTransformsOutgoingMessage::Output>,
    transformer: TTransformsOutgoingMessage,
}

impl<TTransformsOutgoingMessage> ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: Clone + TransformsOutgoingMessage + Send + Sync,
{
    pub const fn new(
        chunk_tx: mpsc::UnboundedSender<TTransformsOutgoingMessage::Output>,
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
        for output in self.transformer.transform(message).await? {
            self.chunk_tx.send(output)?;
        }

        Ok(())
    }
}
