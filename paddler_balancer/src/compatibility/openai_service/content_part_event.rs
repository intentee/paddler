use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Serialize)]
pub struct ContentPartEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub content_index: usize,
    pub part: Value,
}
