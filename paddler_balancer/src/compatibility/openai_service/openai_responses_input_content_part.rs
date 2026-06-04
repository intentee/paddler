use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIResponsesInputContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage {
        #[serde(default)]
        image_url: Option<String>,
    },
    #[serde(other)]
    Unsupported,
}

impl OpenAIResponsesInputContentPart {
    #[must_use]
    pub fn into_conversation_part(self) -> Option<ConversationMessageContentPart> {
        match self {
            Self::InputText { text } => Some(ConversationMessageContentPart::Text { text }),
            Self::InputImage {
                image_url: Some(url),
            } => Some(ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl { url },
            }),
            Self::InputImage { image_url: None } | Self::Unsupported => None,
        }
    }
}
