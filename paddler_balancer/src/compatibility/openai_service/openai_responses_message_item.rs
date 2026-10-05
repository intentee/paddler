use paddler_messaging::conversation_message::ConversationMessage;
use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_message_content::OpenAIResponsesMessageContent;

fn normalize_role(role: String) -> String {
    if role == "developer" {
        "system".to_owned()
    } else {
        role
    }
}

#[derive(Deserialize)]
pub struct OpenAIResponsesMessageItem {
    pub role: String,
    pub content: OpenAIResponsesMessageContent,
}

impl OpenAIResponsesMessageItem {
    #[must_use]
    pub fn into_conversation_message(self) -> ConversationMessage {
        ConversationMessage {
            content: self.content.into_conversation_content(),
            role: normalize_role(self.role),
        }
    }
}
