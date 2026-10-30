use serde_json::from_str;

use paddler_messaging::conversation_tool_call::ConversationToolCall;

use crate::openai_translation_error::OpenAITranslationError;

pub fn conversation_tool_call_from_arguments(
    id: String,
    name: String,
    arguments: &str,
) -> Result<ConversationToolCall, OpenAITranslationError> {
    match from_str(arguments) {
        Ok(arguments) => Ok(ConversationToolCall {
            arguments,
            id,
            name,
        }),
        Err(source) => Err(OpenAITranslationError::ToolCallArgumentsAreNotAJsonObject {
            tool_call_id: id,
            source,
        }),
    }
}
