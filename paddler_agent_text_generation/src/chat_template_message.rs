use llama_cpp_bindings_types::chat_message::ChatMessage;
use llama_cpp_bindings_types::chat_message_content::ChatMessageContent;

use paddler_messaging::constants::ASSISTANT_ROLE;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::image_url::ImageUrl;

use crate::chat_template_message_content::ChatTemplateMessageContent;
use crate::chat_template_tool_call::ChatTemplateToolCall;
use crate::media_marker::MediaMarker;

pub struct ChatTemplateMessage(pub ConversationMessage);

impl ChatTemplateMessage {
    pub fn into_chat_message(
        self,
        media_marker: &MediaMarker,
        image_urls: &mut Vec<ImageUrl>,
        preceding_reasoning_content: Option<String>,
    ) -> ChatMessage {
        match self.0 {
            ConversationMessage::AssistantReasoning { reasoning_content } => ChatMessage::Message {
                role: ASSISTANT_ROLE.to_owned(),
                content: ChatMessageContent::Text(String::new()),
                reasoning_content: Some(reasoning_content),
            },
            ConversationMessage::AssistantToolCalls {
                content,
                tool_calls,
            } => ChatMessage::AssistantToolCalls {
                content: ChatTemplateMessageContent(content)
                    .into_chat_message_content(media_marker, image_urls),
                reasoning_content: preceding_reasoning_content,
                tool_calls: tool_calls
                    .into_iter()
                    .map(|tool_call| ChatTemplateToolCall(tool_call).into_chat_message_tool_call())
                    .collect(),
            },
            ConversationMessage::Message { content, role } => ChatMessage::Message {
                role,
                content: ChatTemplateMessageContent(content)
                    .into_chat_message_content(media_marker, image_urls),
                reasoning_content: preceding_reasoning_content,
            },
            ConversationMessage::ToolResult {
                content,
                tool_call_id,
            } => ChatMessage::ToolResult {
                content: ChatTemplateMessageContent(content)
                    .into_chat_message_content(media_marker, image_urls),
                tool_call_id,
            },
        }
    }
}
