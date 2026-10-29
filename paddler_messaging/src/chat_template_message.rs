use serde::Serialize;

use crate::chat_template_message_content::ChatTemplateMessageContent;
use crate::chat_template_tool_call::ChatTemplateToolCall;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ChatTemplateMessage {
    AssistantToolCalls {
        content: ChatTemplateMessageContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
        role: &'static str,
        tool_calls: Vec<ChatTemplateToolCall>,
    },
    Message {
        content: ChatTemplateMessageContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
        role: String,
    },
    ToolResult {
        content: ChatTemplateMessageContent,
        role: &'static str,
        tool_call_id: String,
    },
}
