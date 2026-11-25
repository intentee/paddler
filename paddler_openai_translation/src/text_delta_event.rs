use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct TextDeltaEvent {
    pub item_id: String,
    pub output_index: usize,
    pub content_index: usize,
    pub delta: String,
}
