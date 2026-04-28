use std::pin::Pin;

use futures_util::Stream;
use paddler_types::inference_client::Message as InferenceMessage;

use crate::Result;

pub type InferenceMessageStream =
    Pin<Box<dyn Stream<Item = Result<InferenceMessage>> + Send + 'static>>;
