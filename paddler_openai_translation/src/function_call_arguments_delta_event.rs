use serde::Serialize;

use crate::encoded_once_text::EncodedOnceText;

#[derive(Clone, Debug, Serialize)]
pub struct FunctionCallArgumentsDeltaEvent {
    pub item_id: String,
    pub output_index: usize,
    pub delta: EncodedOnceText,
}
