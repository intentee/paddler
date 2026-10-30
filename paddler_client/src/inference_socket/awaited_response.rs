use tokio::sync::mpsc::UnboundedSender;

use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::error::Result;

pub struct AwaitedResponse {
    pub response_tx: UnboundedSender<Result<InferenceMessage>>,
    pub stop_frame: String,
}
