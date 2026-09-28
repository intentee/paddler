use paddler_messaging::chat_template_message::ChatTemplateMessage;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

pub struct ChatPromptRenderRequest<'request> {
    pub add_generation_prompt: bool,
    pub enable_thinking: bool,
    pub messages: &'request [ChatTemplateMessage],
    pub tools: &'request [Tool<ValidatedParametersSchema>],
}
