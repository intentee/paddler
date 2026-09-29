use std::fmt::Display;

use futures_util::Sink;
use futures_util::SinkExt as _;
use log::error;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;

async fn write_message<TSink>(write: &mut TSink, message: ManagementJsonRpcMessage) -> bool
where
    TSink: Sink<Message> + Unpin,
    TSink::Error: Display,
{
    match serde_json::to_string(&message) {
        Ok(serialized_message) => {
            match write.send(Message::Text(serialized_message.into())).await {
                Ok(()) => true,
                Err(err) => {
                    error!("Failed to send a management socket message: {err}");

                    false
                }
            }
        }
        Err(err) => {
            error!("Failed to serialize a management socket message: {err}");

            true
        }
    }
}

pub async fn forward_management_socket_messages<TSink>(
    connection_close: CancellationToken,
    mut message_rx: mpsc::UnboundedReceiver<ManagementJsonRpcMessage>,
    mut write: TSink,
) -> TSink
where
    TSink: Sink<Message> + Unpin,
    TSink::Error: Display,
{
    loop {
        tokio::select! {
            () = connection_close.cancelled() => {
                while let Ok(message) = message_rx.try_recv()
                    && write_message(&mut write, message).await
                {}

                break;
            }
            message = message_rx.recv() => {
                let Some(message) = message else {
                    break;
                };

                if !write_message(&mut write, message).await {
                    break;
                }
            }
        }
    }

    write
}

#[cfg(test)]
mod tests {
    use futures::channel::mpsc as sink_channel;
    use futures_util::StreamExt as _;
    use paddler_messaging::management_socket::balancer::notification::Notification;

    use super::*;

    fn deregister_agent() -> ManagementJsonRpcMessage {
        ManagementJsonRpcMessage::Notification(Notification::DeregisterAgent)
    }

    fn deregister_agent_text() -> Message {
        Message::Text(
            serde_json::to_string(&deregister_agent())
                .expect("a deregistration must serialize")
                .into(),
        )
    }

    #[tokio::test]
    async fn forwards_queued_messages_until_every_sender_is_gone() {
        let (sink_tx, sink_rx) = sink_channel::unbounded::<Message>();
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        message_tx
            .send(deregister_agent())
            .expect("the forwarder must still receive messages");
        drop(message_tx);

        drop(
            forward_management_socket_messages(CancellationToken::new(), message_rx, sink_tx).await,
        );

        assert_eq!(
            sink_rx.collect::<Vec<Message>>().await,
            vec![deregister_agent_text()]
        );
    }

    #[tokio::test]
    async fn drains_queued_messages_when_the_connection_closes() {
        let (sink_tx, sink_rx) = sink_channel::unbounded::<Message>();
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let connection_close = CancellationToken::new();

        for _ in 0..2 {
            message_tx
                .send(deregister_agent())
                .expect("the forwarder must still receive messages");
        }
        connection_close.cancel();

        drop(forward_management_socket_messages(connection_close, message_rx, sink_tx).await);

        assert_eq!(
            sink_rx.collect::<Vec<Message>>().await,
            vec![deregister_agent_text(), deregister_agent_text()]
        );
        drop(message_tx);
    }

    #[tokio::test]
    async fn stops_forwarding_when_the_socket_rejects_a_message() {
        let (sink_tx, sink_rx) = sink_channel::unbounded::<Message>();
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        drop(sink_rx);
        message_tx
            .send(deregister_agent())
            .expect("the forwarder must still receive messages");

        let returned_sink =
            forward_management_socket_messages(CancellationToken::new(), message_rx, sink_tx).await;

        assert!(returned_sink.is_closed());
        drop(message_tx);
    }
}
