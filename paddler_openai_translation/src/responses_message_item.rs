use serde::Deserialize;

use paddler_messaging::conversation_message::ConversationMessage;

use crate::responses_message_content::ResponsesMessageContent;

fn normalize_role(role: String) -> String {
    if role == "developer" {
        "system".to_owned()
    } else {
        role
    }
}

#[derive(Deserialize)]
pub struct ResponsesMessageItem {
    pub role: String,
    pub content: ResponsesMessageContent,
}

impl ResponsesMessageItem {
    #[must_use]
    pub fn into_conversation_message(self) -> ConversationMessage {
        ConversationMessage {
            content: self.content.into_conversation_content(),
            role: normalize_role(self.role),
        }
    }
}
