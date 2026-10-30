use serde::Deserialize;

use paddler_messaging::conversation_tool_call::ConversationToolCall;

use crate::conversation_tool_call_from_arguments::conversation_tool_call_from_arguments;
use crate::openai_translation_error::OpenAITranslationError;

#[derive(Deserialize)]
pub struct ResponsesFunctionCallItem {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

impl ResponsesFunctionCallItem {
    pub fn into_conversation_tool_call(
        self,
    ) -> Result<ConversationToolCall, OpenAITranslationError> {
        let Self {
            call_id,
            name,
            arguments,
        } = self;

        conversation_tool_call_from_arguments(call_id, name, &arguments)
    }
}
