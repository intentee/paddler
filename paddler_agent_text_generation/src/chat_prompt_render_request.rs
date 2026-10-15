use serde_json::Value;

use paddler_messaging::chat_template_message::ChatTemplateMessage;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;

pub struct ChatPromptRenderRequest<'request> {
    pub add_generation_prompt: bool,
    pub enable_thinking: bool,
    pub messages: &'request [ChatTemplateMessage],
    pub tools: &'request [Tool<Value>],
}
