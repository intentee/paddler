use std::fmt::Display;

use futures_util::Sink;
use futures_util::SinkExt as _;
use log::error;
use serde::Serialize;
use serde_json::to_string;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::management_connection_end::ManagementConnectionEnd;
use crate::management_connection_step::ManagementConnectionStep;

pub async fn write_management_message<TSink, TMessage>(
    sink: &mut TSink,
    message: &TMessage,
    step_after_write: ManagementConnectionStep,
) -> ManagementConnectionStep
where
    TSink: Sink<Message> + Unpin,
    TSink::Error: Display,
    TMessage: Serialize,
{
    match to_string(message) {
        Ok(serialized_message) => match sink.send(Message::Text(serialized_message.into())).await {
            Ok(()) => step_after_write,
            Err(send_error) => {
                error!("Failed to send a management socket message: {send_error}");

                ManagementConnectionStep::End(ManagementConnectionEnd::OutgoingStreamFailed)
            }
        },
        Err(serialization_error) => {
            error!("Failed to serialize a management socket message: {serialization_error}");

            ManagementConnectionStep::End(ManagementConnectionEnd::OutgoingMessageUnserializable)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use futures::channel::mpsc as sink_channel;
    use tokio_tungstenite::tungstenite::protocol::Message;

    use super::write_management_message;
    use crate::management_connection_end::ManagementConnectionEnd;
    use crate::management_connection_step::ManagementConnectionStep;

    #[tokio::test]
    async fn ends_the_connection_when_the_socket_rejects_the_message() {
        let (mut sink, sink_receiver) = sink_channel::unbounded::<Message>();

        drop(sink_receiver);

        assert!(matches!(
            write_management_message(&mut sink, &"status", ManagementConnectionStep::Continue)
                .await,
            ManagementConnectionStep::End(ManagementConnectionEnd::OutgoingStreamFailed)
        ));
    }

    #[tokio::test]
    async fn ends_the_connection_when_the_message_cannot_be_serialized() {
        let (mut sink, _sink_receiver) = sink_channel::unbounded::<Message>();
        let map_with_a_non_string_key = BTreeMap::from([(vec![1_u8], 1_u8)]);

        assert!(matches!(
            write_management_message(
                &mut sink,
                &map_with_a_non_string_key,
                ManagementConnectionStep::Continue
            )
            .await,
            ManagementConnectionStep::End(ManagementConnectionEnd::OutgoingMessageUnserializable)
        ));
    }
}
