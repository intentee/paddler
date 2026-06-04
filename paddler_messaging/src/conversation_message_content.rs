use serde::Deserialize;
use serde::Serialize;

use crate::conversation_message_content_part::ConversationMessageContentPart;
use crate::image_url::ImageUrl;

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
                    ConversationMessageContentPart::ImageUrl { .. } => None,
                })
                .collect::<Vec<&str>>()
                .join(""),
        }
    }

    #[must_use]
    pub fn image_urls(&self) -> Vec<&ImageUrl> {
        match self {
            Self::Text(_) => vec![],
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ConversationMessageContentPart::ImageUrl { image_url } => Some(image_url),
                    ConversationMessageContentPart::Text { .. } => None,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::conversation_message_content::ConversationMessageContent;
    use crate::conversation_message_content_part::ConversationMessageContentPart;
    use crate::image_url::ImageUrl;

    #[test]
    fn text_content_from_text_variant() {
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
    fn image_urls_from_text_variant_is_empty() {
        let content = ConversationMessageContent::Text("hello".to_owned());

        assert!(content.image_urls().is_empty());
    }

    #[test]
    fn image_urls_from_parts_collects_image_urls() {
        let content = ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::Text {
                text: "hello".to_owned(),
            },
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "http://example.com/a.png".to_owned(),
                },
            },
            ConversationMessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "http://example.com/b.png".to_owned(),
                },
            },
        ]);

        let urls = content.image_urls();

        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].url, "http://example.com/a.png");
        assert_eq!(urls[1].url, "http://example.com/b.png");
    }
}
