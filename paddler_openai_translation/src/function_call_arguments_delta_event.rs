use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct FunctionCallArgumentsDeltaEvent {
    pub item_id: String,
    pub output_index: usize,
    pub delta: String,
}
