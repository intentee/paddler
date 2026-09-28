use std::collections::HashMap;

use paddler_messaging::inference_client::message::Message as InferenceMessage;
use tokio::sync::mpsc::UnboundedSender;

use crate::error::Result;

pub enum PendingRequestsState {
    Closed,
    Open(HashMap<String, UnboundedSender<Result<InferenceMessage>>>),
}
