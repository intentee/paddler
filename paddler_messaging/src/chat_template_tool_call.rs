use serde::Serialize;

use crate::chat_template_tool_call_function::ChatTemplateToolCallFunction;
use crate::conversation_tool_call::ConversationToolCall;

const FUNCTION_TOOL_CALL_TYPE: &str = "function";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTemplateToolCall {
    pub function: ChatTemplateToolCallFunction,
    pub id: String,
    #[serde(rename = "type")]
    pub tool_call_type: &'static str,
}

impl From<ConversationToolCall> for ChatTemplateToolCall {
    fn from(
        ConversationToolCall {
            arguments,
            id,
            name,
        }: ConversationToolCall,
    ) -> Self {
        Self {
            function: ChatTemplateToolCallFunction { arguments, name },
            id,
            tool_call_type: FUNCTION_TOOL_CALL_TYPE,
        }
    }
}
