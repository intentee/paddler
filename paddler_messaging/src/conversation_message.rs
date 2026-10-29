use serde::Deserialize;
use serde::Serialize;

use crate::assistant_role::ASSISTANT_ROLE;
use crate::chat_template_content_form::ChatTemplateContentForm;
use crate::chat_template_message::ChatTemplateMessage;
use crate::chat_template_message_content::ChatTemplateMessageContent;
use crate::conversation_message_content::ConversationMessageContent;
use crate::conversation_tool_call::ConversationToolCall;
use crate::image_url::ImageUrl;
use crate::media_marker::MediaMarker;
use crate::tool_role::TOOL_ROLE;

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
        content_form: ChatTemplateContentForm,
        media_marker: &MediaMarker,
        image_urls: &mut Vec<ImageUrl>,
        preceding_reasoning_content: Option<String>,
    ) -> ChatTemplateMessage {
        match self {
            Self::AssistantReasoning { reasoning_content } => ChatTemplateMessage::Message {
                content: ChatTemplateMessageContent::Text(String::new()),
                reasoning_content: Some(reasoning_content),
                role: ASSISTANT_ROLE.to_owned(),
            },
            Self::AssistantToolCalls {
                content,
                tool_calls,
            } => ChatTemplateMessage::AssistantToolCalls {
                content: content.into_chat_template_content(content_form, media_marker, image_urls),
                reasoning_content: preceding_reasoning_content,
                role: ASSISTANT_ROLE,
                tool_calls: tool_calls.into_iter().map(Into::into).collect(),
            },
            Self::Message { content, role } => ChatTemplateMessage::Message {
                content: content.into_chat_template_content(content_form, media_marker, image_urls),
                reasoning_content: preceding_reasoning_content,
                role,
            },
            Self::ToolResult {
                content,
                tool_call_id,
            } => ChatTemplateMessage::ToolResult {
                content: content.into_chat_template_content(content_form, media_marker, image_urls),
                role: TOOL_ROLE,
                tool_call_id,
            },
        }
    }
}
