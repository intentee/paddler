use serde::Deserialize;

use paddler_messaging::conversation_message_content::ConversationMessageContent;

use crate::responses_input_content_part::ResponsesInputContentPart;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ResponsesMessageContent {
    Text(String),
    Parts(Vec<ResponsesInputContentPart>),
}

impl ResponsesMessageContent {
    #[must_use]
    pub fn into_conversation_content(self) -> ConversationMessageContent {
        match self {
            Self::Text(text) => ConversationMessageContent::Text(text),
            Self::Parts(parts) => ConversationMessageContent::Parts(
                parts
                    .into_iter()
                    .map(ResponsesInputContentPart::into_conversation_part)
                    .collect(),
            ),
        }
    }
}
