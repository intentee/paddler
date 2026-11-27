use serde::Deserialize;
use serde::Serialize;

use crate::conversation_message_content::ConversationMessageContent;
use crate::conversation_role::ConversationRole;
use crate::conversation_tool_call::ConversationToolCall;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum ConversationMessage {
    AssistantReasoning {
        reasoning_content: String,
    },
    AssistantToolCalls {
        content: ConversationMessageContent,
        tool_calls: Vec<ConversationToolCall>,
    },
    Message {
        content: ConversationMessageContent,
        role: String,
    },
    ToolResult {
        content: ConversationMessageContent,
        tool_call_id: String,
    },
}

impl ConversationMessage {
    #[must_use]
    pub fn from_text(role: &str, text: &str) -> Self {
        Self::Message {
            content: ConversationMessageContent::Text(text.to_owned()),
            role: role.to_owned(),
        }
    }

    #[must_use]
    pub fn is_assistant_reply(&self) -> bool {
        match self {
            Self::AssistantToolCalls { .. } => true,
            Self::Message { role, .. } => role == ConversationRole::Assistant.name(),
            Self::AssistantReasoning { .. } | Self::ToolResult { .. } => false,
        }
    }
}
