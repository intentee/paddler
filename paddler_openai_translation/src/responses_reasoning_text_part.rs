use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ResponsesReasoningTextPart {
    #[serde(rename = "reasoning_text")]
    ReasoningText { text: String },
}
