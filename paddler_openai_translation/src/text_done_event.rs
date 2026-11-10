use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct TextDoneEvent {
    pub item_id: String,
    pub output_index: usize,
    pub content_index: usize,
    pub text: String,
}
