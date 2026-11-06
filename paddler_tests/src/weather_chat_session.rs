use llama_cpp_bindings::chat_assistant_turn::ChatAssistantTurn;
use llama_cpp_bindings::chat_session::ChatSession;
use llama_cpp_bindings::chat_session_options::ChatSessionOptions;
use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::chat_tool_choice::ChatToolChoice;
use llama_cpp_bindings::error::chat_session_error::ChatSessionError;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings_types::chat_message::ChatMessage;
use llama_cpp_bindings_types::chat_message_content::ChatMessageContent;
use llama_cpp_bindings_types::chat_tool::ChatTool;
use serde_json::Value;

use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;

use crate::get_weather_tool::get_weather_tool;
use crate::weather_tool_question::WEATHER_TOOL_QUESTION;

pub fn weather_chat_session(
    model: &LlamaModel,
    chat_templates: &ChatTemplates,
) -> Result<ChatSession, ChatSessionError> {
    ChatSession::new(
        model,
        chat_templates,
        &[ChatMessage::Message {
            role: "user".to_owned(),
            content: ChatMessageContent::Text(WEATHER_TOOL_QUESTION.to_owned()),
            reasoning_content: None,
        }],
        &[ChatTool::from(&Tool::<Value>::from(get_weather_tool()))],
        &ChatSessionOptions {
            assistant_turn: ChatAssistantTurn::New,
            enable_thinking: false,
            parallel_tool_calls: true,
            tool_choice: ChatToolChoice::Auto,
        },
    )
}
