use serde::Serialize;

use crate::chat_template_message_content::ChatTemplateMessageContent;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTemplateMessage {
    pub content: ChatTemplateMessageContent,
    pub role: String,
}
