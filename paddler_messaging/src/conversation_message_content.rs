use serde::Deserialize;
use serde::Serialize;

use crate::chat_template_content_form::ChatTemplateContentForm;
use crate::chat_template_message_content::ChatTemplateMessageContent;
use crate::conversation_message_content_part::ConversationMessageContentPart;
use crate::image_url::ImageUrl;
use crate::media_marker::MediaMarker;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ConversationMessageContent {
    Text(String),
    Parts(Vec<ConversationMessageContentPart>),
}

impl ConversationMessageContent {
    pub fn into_chat_template_content(
        self,
        content_form: ChatTemplateContentForm,
        media_marker: &MediaMarker,
        image_urls: &mut Vec<ImageUrl>,
    ) -> ChatTemplateMessageContent {
        match self {
            Self::Text(text) => ChatTemplateMessageContent::Text(text),
            Self::Parts(parts) => content_form.content_of(
                parts
                    .into_iter()
                    .map(|part| match part {
                        ConversationMessageContentPart::Text { text } => text,
                        ConversationMessageContentPart::ImageUrl { image_url } => {
                            image_urls.push(image_url);

                            media_marker.marker.clone()
                        }
                    })
                    .collect(),
            ),
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
    use crate::chat_template_content_form::ChatTemplateContentForm;
    use crate::chat_template_message_content::ChatTemplateMessageContent;
    use crate::conversation_message_content::ConversationMessageContent;
    use crate::conversation_message_content_part::ConversationMessageContentPart;
    use crate::image_url::ImageUrl;
    use crate::media_marker::MediaMarker;

    #[test]
    fn a_template_taking_text_receives_the_text_parts_joined_by_newlines() {
        let content = ConversationMessageContent::Parts(vec![
            ConversationMessageContentPart::Text {
                text: "first".to_owned(),
            },
            ConversationMessageContentPart::Text {
                text: "second".to_owned(),
            },
        ]);

        assert_eq!(
            content.into_chat_template_content(
                ChatTemplateContentForm::JoinedText,
                &MediaMarker::new("[IMAGE]".to_owned()),
                &mut Vec::new(),
            ),
            ChatTemplateMessageContent::Text("first\nsecond".to_owned())
        );
    }

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

        assert_eq!(content.image_urls(), Vec::<&ImageUrl>::new());
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
