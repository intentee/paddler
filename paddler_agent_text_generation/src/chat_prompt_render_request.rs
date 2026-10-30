use serde_json::Value;

use paddler_messaging::chat_template_message::ChatTemplateMessage;

pub struct ChatPromptRenderRequest<'request> {
    pub add_generation_prompt: bool,
    pub enable_thinking: bool,
    pub messages: &'request [ChatTemplateMessage],
    pub tools: &'request Value,
}
