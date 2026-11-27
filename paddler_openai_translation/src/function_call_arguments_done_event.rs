use serde::Serialize;

use crate::encoded_once_text::EncodedOnceText;

#[derive(Clone, Debug, Serialize)]
pub struct FunctionCallArgumentsDoneEvent {
    pub item_id: String,
    pub output_index: usize,
    pub name: String,
    pub arguments: EncodedOnceText,
}
