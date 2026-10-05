use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ResponsesReasoningPart {
    ReasoningText { text: String },
}
