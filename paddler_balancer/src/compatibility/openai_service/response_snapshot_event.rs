use serde::Serialize;

use crate::compatibility::openai_service::responses_response::ResponsesResponse;

#[derive(Clone, Debug, Serialize)]
pub struct ResponseSnapshotEvent {
    pub sequence_number: u64,
    pub response: ResponsesResponse,
}
