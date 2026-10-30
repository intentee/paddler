use serde::Deserialize;

use paddler_messaging::conversation_message_content::ConversationMessageContent;

use crate::responses_assistant_content_part::ResponsesAssistantContentPart;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ResponsesAssistantMessageContent {
    Text(String),
    Parts(Vec<ResponsesAssistantContentPart>),
}

impl ResponsesAssistantMessageContent {
    #[must_use]
    pub fn into_conversation_content(self) -> ConversationMessageContent {
        match self {
            Self::Text(text) => ConversationMessageContent::Text(text),
            Self::Parts(parts) => ConversationMessageContent::Parts(
                parts
                    .into_iter()
                    .map(ResponsesAssistantContentPart::into_conversation_part)
                    .collect(),
            ),
        }
    }
}
