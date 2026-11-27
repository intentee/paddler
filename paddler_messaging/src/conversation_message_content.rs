use serde::Deserialize;
use serde::Serialize;

use crate::conversation_message_content_part::ConversationMessageContentPart;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ConversationMessageContent {
    Text(String),
    Parts(Vec<ConversationMessageContentPart>),
}

impl ConversationMessageContent {
    #[must_use]
    pub fn text_content(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ConversationMessageContentPart::Text { text } => Some(text.as_str()),
                    ConversationMessageContentPart::ImageUrl { .. }
                    | ConversationMessageContentPart::InputAudio { .. } => None,
                })
                .collect::<Vec<&str>>()
                .join(""),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::audio_format::AudioFormat;
    use crate::conversation_message_content::ConversationMessageContent;
    use crate::conversation_message_content_part::ConversationMessageContentPart;
    use crate::image_url::ImageUrl;
    use crate::input_audio::InputAudio;

    #[test]
    fn text_content_of_a_text_variant_is_its_text() {
        let content = ConversationMessageContent::Text("hello world".to_owned());

        assert_eq!(content.text_content(), "hello world");
    }

    #[test]
    fn text_content_from_parts_joins_text_and_skips_media() {
        let content = ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::Text {
                text: "hello ".to_owned(),
            },
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "http://example.com/img.png".to_owned(),
                },
            },
            ConversationMessageContentPart::InputAudio {
                input_audio: InputAudio {
                    data: "UklGRg==".to_owned(),
                    format: AudioFormat::Wav,
                },
            },
            ConversationMessageContentPart::Text {
                text: "world".to_owned(),
            },
        ]);

        assert_eq!(content.text_content(), "hello world");
    }
}
