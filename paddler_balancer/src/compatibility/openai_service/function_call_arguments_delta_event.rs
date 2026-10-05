#[derive(Clone, Debug)]
pub struct FunctionCallArgumentsDeltaEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub delta: String,
}
