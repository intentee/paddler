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

#[cfg(test)]
mod tests {
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;

    use super::OpenAIResponsesInputContentPart;

    #[test]
    fn input_text_becomes_text_part() {
        assert!(matches!(
            OpenAIResponsesInputContentPart::InputText {
                text: "hello".to_owned(),
            }
            .into_conversation_part(),
            Some(ConversationMessageContentPart::Text { text }) if text == "hello"
        ));
    }

    #[test]
    fn input_image_with_url_becomes_image_part() {
        assert!(matches!(
            OpenAIResponsesInputContentPart::InputImage {
                image_url: Some("https://example.test/cat.png".to_owned()),
            }
            .into_conversation_part(),
            Some(ConversationMessageContentPart::ImageUrl { image_url })
                if image_url.url == "https://example.test/cat.png"
        ));
    }

    #[test]
    fn input_image_without_url_becomes_none() {
        assert!(
            OpenAIResponsesInputContentPart::InputImage { image_url: None }
                .into_conversation_part()
                .is_none()
        );
    }

    #[test]
    fn unsupported_becomes_none() {
        assert!(
            OpenAIResponsesInputContentPart::Unsupported
                .into_conversation_part()
                .is_none()
        );
    }
}
