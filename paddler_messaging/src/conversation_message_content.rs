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
    pub fn contains_image_url(&self) -> bool {
        match self {
            Self::Text(_) => false,
            Self::Parts(parts) => parts
                .iter()
                .any(|part| matches!(part, ConversationMessageContentPart::ImageUrl { .. })),
        }
    }

    #[must_use]
    pub fn text_content(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ConversationMessageContentPart::Text { text } => Some(text.as_str()),
                    ConversationMessageContentPart::ImageUrl { .. } => None,
                })
                .collect::<Vec<&str>>()
                .join(""),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::conversation_message_content::ConversationMessageContent;
    use crate::conversation_message_content_part::ConversationMessageContentPart;
    use crate::image_url::ImageUrl;

    #[test]
    fn text_content_of_a_text_variant_is_its_text() {
        let content = ConversationMessageContent::Text("hello world".to_owned());

        assert_eq!(content.text_content(), "hello world");
    }

    #[test]
    fn text_content_from_parts_joins_text_and_skips_images() {
        let content = ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::Text {
                text: "hello ".to_owned(),
            },
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "http://example.com/img.png".to_owned(),
                },
            },
            ConversationMessageContentPart::Text {
                text: "world".to_owned(),
            },
        ]);

        assert_eq!(content.text_content(), "hello world");
    }

    #[test]
    fn text_content_contains_no_image_url() {
        assert!(!ConversationMessageContent::Text("hello".to_owned()).contains_image_url());
    }

    #[test]
    fn parts_with_only_text_contain_no_image_url() {
        let content =
            ConversationMessageContent::Parts(vec![ConversationMessageContentPart::Text {
                text: "hello".to_owned(),
            }]);

        assert!(!content.contains_image_url());
    }

    #[test]
    fn parts_with_an_image_contain_an_image_url() {
        let content = ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::Text {
                text: "hello".to_owned(),
            },
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "http://example.com/a.png".to_owned(),
                },
            },
        ]);

        assert!(content.contains_image_url());
    }
}
