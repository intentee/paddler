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

#[cfg(test)]
mod tests {
    use super::OpenAIResponsesMessageItem;
    use crate::compatibility::openai_service::openai_responses_message_content::OpenAIResponsesMessageContent;

    fn item_with_role(role: &str) -> OpenAIResponsesMessageItem {
        OpenAIResponsesMessageItem {
            role: role.to_owned(),
            content: OpenAIResponsesMessageContent::Text("body".to_owned()),
        }
    }

    #[test]
    fn developer_role_is_normalized_to_system() {
        assert_eq!(
            item_with_role("developer").into_conversation_message().role,
            "system"
        );
    }

    #[test]
    fn other_roles_are_preserved() {
        assert_eq!(
            item_with_role("user").into_conversation_message().role,
            "user"
        );
    }
}
