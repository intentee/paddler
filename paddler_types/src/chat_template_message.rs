use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTemplateMessage {
    pub content: String,
    pub role: String,
}
