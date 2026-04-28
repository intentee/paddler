use std::sync::Arc;

use dashmap::DashMap;
use paddler_types::inference_client::Message as InferenceMessage;
use tokio::sync::mpsc::UnboundedSender;

use crate::error::Result;

pub type PendingRequests = Arc<DashMap<String, UnboundedSender<Result<InferenceMessage>>>>;
