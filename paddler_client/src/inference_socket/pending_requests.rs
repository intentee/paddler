use std::sync::Arc;

use dashmap::DashMap;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use tokio::sync::mpsc::UnboundedSender;

use crate::error::Result;

pub type PendingRequests = Arc<DashMap<String, UnboundedSender<Result<InferenceMessage>>>>;
