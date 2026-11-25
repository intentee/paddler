use serde::Serialize;

use crate::encoded_once_text::EncodedOnceText;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ResponsesContentPart {
    OutputText {
        annotations: [u8; 0],
        logprobs: [u8; 0],
        text: EncodedOnceText,
    },
}

impl ResponsesContentPart {
    #[must_use]
    pub const fn output_text(text: EncodedOnceText) -> Self {
        Self::OutputText {
            annotations: [],
            logprobs: [],
            text,
        }
    }
}
