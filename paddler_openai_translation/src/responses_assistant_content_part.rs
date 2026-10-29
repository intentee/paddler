use serde::Deserialize;

use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ResponsesAssistantContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "output_text")]
    OutputText { text: String },
}

impl ResponsesAssistantContentPart {
    #[must_use]
    pub fn into_conversation_part(self) -> ConversationMessageContentPart {
        match self {
            Self::InputText { text } | Self::OutputText { text } => {
                ConversationMessageContentPart::Text { text }
            }
        }
    }
}
