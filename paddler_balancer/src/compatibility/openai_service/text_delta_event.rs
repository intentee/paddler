#[derive(Clone, Debug)]
pub struct TextDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub content_index: usize,
    pub delta: String,
}
