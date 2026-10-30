use serde::Deserialize;

use paddler_messaging::conversation_message::ConversationMessage;

use crate::openai_translation_error::OpenAITranslationError;
use crate::responses_reasoning_text_part::ResponsesReasoningTextPart;

#[derive(Deserialize)]
pub struct ResponsesReasoningItem {
    pub content: Vec<ResponsesReasoningTextPart>,
}

impl ResponsesReasoningItem {
    pub fn into_conversation_message(self) -> Result<ConversationMessage, OpenAITranslationError> {
        if self.content.is_empty() {
            return Err(OpenAITranslationError::ReasoningItemWithoutContent);
        }

        Ok(ConversationMessage::AssistantReasoning {
            reasoning_content: self
                .content
                .into_iter()
                .map(|ResponsesReasoningTextPart::ReasoningText { text }| text)
                .collect(),
        })
    }
}
