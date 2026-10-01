use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::Stream;

use paddler_client::error::Result as ClientResult;
use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::connection_reservation::ConnectionReservation;

pub struct BudgetedInferenceMessageStream {
    pub connection_reservation: ConnectionReservation,
    pub inference_message_stream: InferenceMessageStream,
}

impl Stream for BudgetedInferenceMessageStream {
    type Item = ClientResult<InferenceMessage>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut()
            .inference_message_stream
            .as_mut()
            .poll_next(context)
    }
}
