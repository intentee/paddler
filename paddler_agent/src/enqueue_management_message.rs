use log::error;
use serde::Serialize;
use serde_json::to_string;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::enqueue_outbound_frame::enqueue_outbound_frame;
use crate::management_connection_end::ManagementConnectionEnd;
use crate::management_connection_step::ManagementConnectionStep;

pub fn enqueue_management_message<TMessage>(
    outbound_tx: &mpsc::UnboundedSender<Message>,
    message: &TMessage,
    step_after_enqueue: ManagementConnectionStep,
) -> ManagementConnectionStep
where
    TMessage: Serialize,
{
    match to_string(message) {
        Ok(serialized_message) => enqueue_outbound_frame(
            outbound_tx,
            Message::Text(serialized_message.into()),
            step_after_enqueue,
        ),
        Err(serialization_error) => {
            error!("Failed to serialize a management socket message: {serialization_error}");

            ManagementConnectionStep::End(ManagementConnectionEnd::OutgoingMessageUnserializable)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tokio::sync::mpsc;

    use super::enqueue_management_message;
    use crate::management_connection_end::ManagementConnectionEnd;
    use crate::management_connection_step::ManagementConnectionStep;

    #[test]
    fn ends_the_connection_when_the_message_cannot_be_serialized() {
        let (outbound_tx, _outbound_rx) = mpsc::unbounded_channel();
        let map_with_a_non_string_key = BTreeMap::from([(vec![1_u8], 1_u8)]);

        assert_eq!(
            enqueue_management_message(
                &outbound_tx,
                &map_with_a_non_string_key,
                ManagementConnectionStep::Continue
            )
            .outcome(),
            Err(&ManagementConnectionEnd::OutgoingMessageUnserializable)
        );
    }
}
