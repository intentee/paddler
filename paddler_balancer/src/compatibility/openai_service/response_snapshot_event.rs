use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
pub struct ResponseSnapshotEvent {
    pub sequence_number: u64,
    pub response: Value,
}
