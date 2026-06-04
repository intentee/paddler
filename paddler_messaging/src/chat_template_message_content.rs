use serde::Serialize;

use crate::chat_template_message_content_part::ChatTemplateMessageContentPart;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ChatTemplateMessageContent {
    Text(String),
    Parts(Vec<ChatTemplateMessageContentPart>),
}
