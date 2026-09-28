use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTemplateMessageContentPart {
    #[serde(rename = "type")]
    pub content_type: &'static str,
    pub text: String,
}
