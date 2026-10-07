use serde::Deserialize;

use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ResponsesInputContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { image_url: String },
}

impl ResponsesInputContentPart {
    #[must_use]
    pub fn into_conversation_part(self) -> ConversationMessageContentPart {
        match self {
            Self::InputText { text } => ConversationMessageContentPart::Text { text },
            Self::InputImage { image_url } => ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl { url: image_url },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;

    use super::ResponsesInputContentPart;

    #[test]
    fn input_text_becomes_text_part() {
        assert!(matches!(
            ResponsesInputContentPart::InputText {
                text: "hello".to_owned(),
            }
            .into_conversation_part(),
            ConversationMessageContentPart::Text { text } if text == "hello"
        ));
    }

    #[test]
    fn input_image_becomes_image_part() {
        assert!(matches!(
            ResponsesInputContentPart::InputImage {
                image_url: "https://example.test/cat.png".to_owned(),
            }
            .into_conversation_part(),
            ConversationMessageContentPart::ImageUrl { image_url }
                if image_url.url == "https://example.test/cat.png"
        ));
    }
}
