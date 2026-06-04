use std::pin::Pin;

use anyhow::Result;
use futures_util::Stream;
use paddler_messaging::inference_client::message::Message as InferenceMessage;

pub type InferenceMessageStream = Pin<Box<dyn Stream<Item = Result<InferenceMessage>> + Send>>;
