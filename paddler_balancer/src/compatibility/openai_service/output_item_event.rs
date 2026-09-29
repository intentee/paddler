use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
pub struct OutputItemEvent {
    pub sequence_number: u64,
    pub output_index: usize,
    pub item: Value,
}
