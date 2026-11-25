pub mod identity_transformer;
pub mod transforms_outgoing_message;

use std::future::ready;

use tokio::sync::mpsc;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;

use self::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::balancer_error::BalancerError;
use crate::controls_session::ControlsSession;
use crate::transform_result::TransformResult;

pub struct ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: TransformsOutgoingMessage + Send + Sync,
{
    chunk_tx: mpsc::UnboundedSender<TransformResult>,
    transformer: TTransformsOutgoingMessage,
}

impl<TTransformsOutgoingMessage> ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: TransformsOutgoingMessage + Send + Sync,
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

    fn forward_transformed(&self, message: OutgoingMessage) -> Result<(), BalancerError> {
        for transform_result in self.transformer.transform(message) {
            self.chunk_tx
                .send(transform_result)
                .map_err(BalancerError::ChunkReceiverClosed)?;
        }

        Ok(())
    }
}

impl<TTransformsOutgoingMessage> ControlsSession<OutgoingMessage>
    for ChunkForwardingSessionController<TTransformsOutgoingMessage>
where
    TTransformsOutgoingMessage: TransformsOutgoingMessage + Send + Sync,
{
    fn send_response(
        &mut self,
        message: OutgoingMessage,
    ) -> impl Future<Output = Result<(), BalancerError>> + Send {
        ready(self.forward_transformed(message))
    }
}
