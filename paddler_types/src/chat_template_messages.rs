use serde::Serialize;

use crate::chat_template_message::ChatTemplateMessage;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTemplateMessages {
    pub messages: Vec<ChatTemplateMessage>,
}
