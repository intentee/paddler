use serde::Deserialize;

use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_role::ConversationRole;

use crate::responses_assistant_message_content::ResponsesAssistantMessageContent;
use crate::responses_message_content::ResponsesMessageContent;

#[derive(Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ResponsesMessageItem {
    Assistant {
        content: ResponsesAssistantMessageContent,
    },
    Developer {
        content: ResponsesMessageContent,
    },
    System {
        content: ResponsesMessageContent,
    },
    User {
        content: ResponsesMessageContent,
    },
}

impl ResponsesMessageItem {
    #[must_use]
    pub fn into_conversation_message(self) -> ConversationMessage {
        match self {
            Self::Assistant { content } => ConversationMessage::Message {
                content: content.into_conversation_content(),
                role: ConversationRole::Assistant.name().to_owned(),
            },
            Self::Developer { content } | Self::System { content } => {
                ConversationMessage::Message {
                    content: content.into_conversation_content(),
                    role: ConversationRole::System.name().to_owned(),
                }
            }
            Self::User { content } => ConversationMessage::Message {
                content: content.into_conversation_content(),
                role: ConversationRole::User.name().to_owned(),
            },
        }
    }
}
