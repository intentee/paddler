use paddler_messaging::conversation_message_content::ConversationMessageContent;
use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_input_content_part::OpenAIResponsesInputContentPart;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum OpenAIResponsesMessageContent {
    Text(String),
    Parts(Vec<OpenAIResponsesInputContentPart>),
}

impl OpenAIResponsesMessageContent {
    #[must_use]
    pub fn into_conversation_content(self) -> ConversationMessageContent {
        match self {
            Self::Text(text) => ConversationMessageContent::Text(text),
            Self::Parts(parts) => ConversationMessageContent::Parts(
                parts
                    .into_iter()
                    .filter_map(OpenAIResponsesInputContentPart::into_conversation_part)
                    .collect(),
            ),
        }
    }
}
