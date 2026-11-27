use llama_cpp_bindings_types::chat_message::ChatMessage;
use llama_cpp_bindings_types::chat_message_content::ChatMessageContent;
use llama_cpp_chat::chat_assistant_turn::ChatAssistantTurn;
use llama_cpp_chat::chat_error::ChatError;
use llama_cpp_chat::chat_session::ChatSession;
use llama_cpp_chat::chat_session_options::ChatSessionOptions;
use llama_cpp_chat::chat_tool_choice::ChatToolChoice;
use llama_cpp_model::chat_templates::ChatTemplates;
use llama_cpp_model::llama_model::LlamaModel;
use serde_json::Value;

use paddler_agent_text_generation::chat_template_tool::ChatTemplateTool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_request_params_fixture::weather_tool_question::weather_tool_question;

use crate::get_weather_tool::get_weather_tool;

/// # Errors
///
/// Returns the [`ChatError`] of a chat template that cannot render the weather question with its tool.
pub fn weather_chat_session(
    model: &LlamaModel,
    chat_templates: &ChatTemplates,
) -> Result<ChatSession, ChatError> {
    ChatSession::new(
        model,
        chat_templates,
        &[ChatMessage::Message {
            role: "user".to_owned(),
            content: ChatMessageContent::Text(weather_tool_question()),
            reasoning_content: None,
        }],
        &[ChatTemplateTool(Tool::<Value>::from(get_weather_tool())).into_chat_tool()],
        &ChatSessionOptions {
            assistant_turn: ChatAssistantTurn::New,
            enable_thinking: false,
            parallel_tool_calls: true,
            tool_choice: ChatToolChoice::Auto,
        },
    )
}
