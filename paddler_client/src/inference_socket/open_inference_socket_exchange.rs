use std::collections::HashMap;

use crate::inference_socket::awaited_response::AwaitedResponse;

#[derive(Default)]
pub struct OpenInferenceSocketExchange {
    pub awaiting_responses: HashMap<String, AwaitedResponse>,
    pub queued_frames: Vec<String>,
}
