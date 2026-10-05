use std::pin::Pin;

use futures_util::Stream;
use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::error::Result;

pub type InferenceMessageStream =
    Pin<Box<dyn Stream<Item = Result<InferenceMessage>> + Send + 'static>>;
