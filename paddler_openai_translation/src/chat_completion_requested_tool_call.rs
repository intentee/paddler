use serde::Deserialize;

use paddler_messaging::conversation_tool_call::ConversationToolCall;

use crate::chat_completion_requested_function::ChatCompletionRequestedFunction;
use crate::conversation_tool_call_from_arguments::conversation_tool_call_from_arguments;
use crate::openai_translation_error::OpenAITranslationError;

#[derive(Deserialize)]
pub struct ChatCompletionRequestedToolCall {
    pub function: ChatCompletionRequestedFunction,
    pub id: String,
}

impl ChatCompletionRequestedToolCall {
    pub fn into_conversation_tool_call(
        self,
    ) -> Result<ConversationToolCall, OpenAITranslationError> {
        let Self {
            function: ChatCompletionRequestedFunction { arguments, name },
            id,
        } = self;

        conversation_tool_call_from_arguments(id, name, &arguments)
    }
}
