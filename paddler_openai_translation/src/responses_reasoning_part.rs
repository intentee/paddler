use serde::Serialize;

use crate::encoded_once_text::EncodedOnceText;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ResponsesReasoningPart {
    ReasoningText { text: EncodedOnceText },
}
