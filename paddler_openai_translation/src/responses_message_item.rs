use serde::Deserialize;

use paddler_messaging::constants::ASSISTANT_ROLE;
use paddler_messaging::constants::SYSTEM_ROLE;
use paddler_messaging::constants::USER_ROLE;
use paddler_messaging::conversation_message::ConversationMessage;

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
                role: ASSISTANT_ROLE.to_owned(),
            },
            Self::Developer { content } | Self::System { content } => {
                ConversationMessage::Message {
                    content: content.into_conversation_content(),
                    role: SYSTEM_ROLE.to_owned(),
                }
            }
            Self::User { content } => ConversationMessage::Message {
                content: content.into_conversation_content(),
                role: USER_ROLE.to_owned(),
            },
        }
    }
}
