use serde::Serialize;

use crate::responses_response::ResponsesResponse;

#[derive(Clone, Debug, Serialize)]
pub struct ResponseSnapshotEvent {
    pub response: ResponsesResponse,
}
