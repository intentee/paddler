use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ResponseSnapshotEvent {
    pub sequence_number: u64,
    pub response: Value,
}
