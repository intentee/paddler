use llama_cpp_bindings_types::chat_message::ChatMessage;
use llama_cpp_bindings_types::chat_message_content::ChatMessageContent;
use serde::Deserialize;
use serde::Serialize;

use crate::assistant_role::ASSISTANT_ROLE;
use crate::conversation_message_content::ConversationMessageContent;
use crate::conversation_tool_call::ConversationToolCall;
use crate::image_url::ImageUrl;
use crate::media_marker::MediaMarker;

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
    pub fn is_assistant_reply(&self) -> bool {
        match self {
            Self::AssistantToolCalls { .. } => true,
            Self::Message { role, .. } => role == ASSISTANT_ROLE,
            Self::AssistantReasoning { .. } | Self::ToolResult { .. } => false,
        }
    }

    pub fn into_chat_template_message(
        self,
        media_marker: &MediaMarker,
        image_urls: &mut Vec<ImageUrl>,
        preceding_reasoning_content: Option<String>,
    ) -> ChatMessage {
        match self {
            Self::AssistantReasoning { reasoning_content } => ChatMessage::Message {
                role: ASSISTANT_ROLE.to_owned(),
                content: ChatMessageContent::Text(String::new()),
                reasoning_content: Some(reasoning_content),
            },
            Self::AssistantToolCalls {
                content,
                tool_calls,
            } => ChatMessage::AssistantToolCalls {
                content: content.into_chat_template_content(media_marker, image_urls),
                reasoning_content: preceding_reasoning_content,
                tool_calls: tool_calls.into_iter().map(Into::into).collect(),
            },
            Self::Message { content, role } => ChatMessage::Message {
                role,
                content: content.into_chat_template_content(media_marker, image_urls),
                reasoning_content: preceding_reasoning_content,
            },
            Self::ToolResult {
                content,
                tool_call_id,
            } => ChatMessage::ToolResult {
                content: content.into_chat_template_content(media_marker, image_urls),
                tool_call_id,
            },
        }
    }
}
