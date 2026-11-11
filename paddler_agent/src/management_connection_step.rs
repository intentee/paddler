use futures_util::stream::BoxStream;
use log::error;
use serde::Serialize;
use serde_json::to_string;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio_tungstenite::tungstenite::protocol::Message;

use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;

use crate::management_connection_end::ManagementConnectionEnd;

pub enum ManagementConnectionStep {
    Continue,
    Deregister,
    End(ManagementConnectionEnd),
    StreamResponses(BoxStream<'static, ManagementJsonRpcMessage>),
    Write(ManagementJsonRpcMessage),
}

impl ManagementConnectionStep {
    #[must_use]
    pub fn after_enqueuing_management_message<TMessage>(
        self,
        outbound_tx: &mpsc::UnboundedSender<Message>,
        message: &TMessage,
    ) -> Self
    where
        TMessage: Serialize,
    {
        match to_string(message) {
            Ok(serialized_message) => self.after_enqueuing_outbound_frame(
                outbound_tx,
                Message::Text(serialized_message.into()),
            ),
            Err(serialization_error) => {
                error!("Failed to serialize a management socket message: {serialization_error}");

                Self::End(ManagementConnectionEnd::OutgoingMessageUnserializable)
            }
        }
    }

    #[must_use]
    pub fn after_enqueuing_outbound_frame(
        self,
        outbound_tx: &mpsc::UnboundedSender<Message>,
        frame: Message,
    ) -> Self {
        match outbound_tx.send(frame) {
            Ok(()) => self,
            Err(SendError(undelivered_frame)) => {
                error!(
                    "The management socket writer stopped before it could send {undelivered_frame}"
                );

                Self::End(ManagementConnectionEnd::OutgoingStreamFailed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::Bytes;
    use tokio_tungstenite::tungstenite::protocol::Message;

    use super::ManagementConnectionStep;
    use crate::management_connection_end::ManagementConnectionEnd;

    #[test]
    fn ends_the_connection_when_the_message_cannot_be_serialized() {
        let (outbound_tx, _outbound_rx) = mpsc::unbounded_channel();
        let map_with_a_non_string_key = BTreeMap::from([(vec![1_u8], 1_u8)]);

        assert!(matches!(
            ManagementConnectionStep::Continue
                .after_enqueuing_management_message(&outbound_tx, &map_with_a_non_string_key),
            ManagementConnectionStep::End(management_connection_end)
                if management_connection_end == ManagementConnectionEnd::OutgoingMessageUnserializable
        ));
    }

    #[test]
    fn ends_the_connection_when_the_writer_stopped() {
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel::<Message>();

        drop(outbound_rx);

        assert!(matches!(
            ManagementConnectionStep::Continue
                .after_enqueuing_outbound_frame(&outbound_tx, Message::Ping(Bytes::new())),
            ManagementConnectionStep::End(management_connection_end)
                if management_connection_end == ManagementConnectionEnd::OutgoingStreamFailed
        ));
    }
}
