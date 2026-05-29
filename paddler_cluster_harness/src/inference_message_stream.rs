use std::pin::Pin;

use anyhow::Result;
use futures_util::Stream;
use paddler::balancer::inference_client::Message as InferenceMessage;

pub type InferenceMessageStream = Pin<Box<dyn Stream<Item = Result<InferenceMessage>> + Send>>;
