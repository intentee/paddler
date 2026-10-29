use std::future::Future;
use std::future::ready;
use std::marker::PhantomData;

use anyhow::Result;
use serde_json::to_string;
use tokio::sync::mpsc::UnboundedSender;

use paddler_messaging::rpc_message::RpcMessage;

use crate::controls_session::ControlsSession;
use crate::websocket_outbound_frame::WebSocketOutboundFrame;

pub struct WebSocketSessionController<TResponse>
where
    TResponse: RpcMessage + Sync,
{
    outbound_tx: UnboundedSender<WebSocketOutboundFrame>,
    _marker: PhantomData<TResponse>,
}

impl<TResponse> WebSocketSessionController<TResponse>
where
    TResponse: RpcMessage + Sync,
{
    #[must_use]
    pub const fn new(outbound_tx: UnboundedSender<WebSocketOutboundFrame>) -> Self {
        Self {
            outbound_tx,
            _marker: PhantomData,
        }
    }

    fn send_text_frame(&self, message: &TResponse) -> Result<()> {
        self.outbound_tx
            .send(WebSocketOutboundFrame::Text(to_string(message)?))?;

        Ok(())
    }
}

impl<TResponse> ControlsSession<TResponse> for WebSocketSessionController<TResponse>
where
    TResponse: RpcMessage + Sync + 'static,
{
    fn send_response(&mut self, message: TResponse) -> impl Future<Output = Result<()>> + Send {
        ready(self.send_text_frame(&message))
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde::Serializer;
    use serde::ser::Error as _;
    use serde_json::to_string;
    use tokio::sync::mpsc::error::SendError;
    use tokio::sync::mpsc::error::TryRecvError;
    use tokio::sync::mpsc::unbounded_channel;

    use paddler_messaging::inference_client::message::Message as OutgoingMessage;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::rpc_message::RpcMessage;

    use super::WebSocketSessionController;
    use crate::controls_session::ControlsSession as _;
    use crate::websocket_outbound_frame::WebSocketOutboundFrame;

    struct MessageThatCannotSerialize;

    impl Serialize for MessageThatCannotSerialize {
        fn serialize<TSerializer>(
            &self,
            _serializer: TSerializer,
        ) -> Result<TSerializer::Ok, TSerializer::Error>
        where
            TSerializer: Serializer,
        {
            Err(TSerializer::Error::custom(
                "the message cannot be serialized",
            ))
        }
    }

    impl RpcMessage for MessageThatCannotSerialize {}

    fn request_rejection() -> OutgoingMessage {
        OutgoingMessage::Error(ErrorEnvelope {
            request_id: "request".to_owned(),
            error: JsonRpcError {
                code: 400,
                description: "rejected".to_owned(),
            },
        })
    }

    #[tokio::test]
    async fn a_response_for_a_writer_that_stopped_is_reported_as_undelivered() {
        let (outbound_tx, outbound_rx) = unbounded_channel();

        drop(outbound_rx);

        let SendError(undelivered_frame) =
            WebSocketSessionController::<OutgoingMessage>::new(outbound_tx)
                .send_response(request_rejection())
                .await
                .unwrap_err()
                .downcast::<SendError<WebSocketOutboundFrame>>()
                .unwrap();

        assert_eq!(
            undelivered_frame,
            WebSocketOutboundFrame::Text(to_string(&request_rejection()).unwrap())
        );
    }

    #[tokio::test]
    async fn a_response_that_cannot_be_serialized_never_reaches_the_writer() {
        let (outbound_tx, mut outbound_rx) = unbounded_channel();

        assert_eq!(
            WebSocketSessionController::<MessageThatCannotSerialize>::new(outbound_tx)
                .send_response(MessageThatCannotSerialize)
                .await
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some("the message cannot be serialized".to_owned())
        );
        assert_eq!(outbound_rx.try_recv(), Err(TryRecvError::Disconnected));
    }
}
