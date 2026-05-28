use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTemplateMessageContentPart {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}
