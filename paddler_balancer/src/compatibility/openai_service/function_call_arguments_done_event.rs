#[derive(Clone, Debug)]
pub struct FunctionCallArgumentsDoneEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub name: String,
    pub arguments: String,
}
