use serde::Deserialize;
use serde::Serialize;

use crate::chat_template_message::ChatTemplateMessage;
use crate::chat_template_message_content::ChatTemplateMessageContent;
use crate::chat_template_message_content_part::ChatTemplateMessageContentPart;
use crate::chat_template_messages::ChatTemplateMessages;
use crate::conversation_message::ConversationMessage;
use crate::conversation_message_content::ConversationMessageContent;
use crate::conversation_message_content_part::ConversationMessageContentPart;
use crate::image_url::ImageUrl;
use crate::media_marker::MediaMarker;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ConversationHistory {
    pub messages: Vec<ConversationMessage>,
}

impl ConversationHistory {
    pub fn new(messages: Vec<ConversationMessage>) -> Self {
        Self { messages }
    }

    pub fn extract_image_urls(&self) -> Vec<&ImageUrl> {
        self.messages
            .iter()
            .flat_map(|message| message.content.image_urls())
            .collect()
    }

    pub fn replace_images_with_marker(&self, media_marker: &MediaMarker) -> ChatTemplateMessages {
        let marker_string = media_marker.to_string();

        ChatTemplateMessages {
            messages: self
                .messages
                .iter()
                .map(|message| ChatTemplateMessage {
                    content: match &message.content {
                        ConversationMessageContent::Text(text) => {
                            ChatTemplateMessageContent::Text(text.clone())
                        }
                        ConversationMessageContent::Parts(parts) => {
                            ChatTemplateMessageContent::Parts(
                                parts
                                    .iter()
                                    .map(|part| match part {
                                        ConversationMessageContentPart::Text { text } => {
                                            ChatTemplateMessageContentPart {
                                                content_type: "text".to_string(),
                                                text: text.clone(),
                                            }
                                        }
                                        ConversationMessageContentPart::ImageUrl { .. } => {
                                            ChatTemplateMessageContentPart {
                                                content_type: "text".to_string(),
                                                text: marker_string.clone(),
                                            }
                                        }
                                    })
                                    .collect(),
                            )
                        }
                    },
                    role: message.role.clone(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_text_message(role: &str, text: &str) -> ConversationMessage {
        ConversationMessage {
            content: ConversationMessageContent::Text(text.to_string()),
            role: role.to_string(),
        }
    }

    fn make_parts_message(
        role: &str,
        parts: Vec<ConversationMessageContentPart>,
    ) -> ConversationMessage {
        ConversationMessage {
            content: ConversationMessageContent::Parts(parts),
            role: role.to_string(),
        }
    }

    #[test]
    fn extract_image_urls_from_mixed_content() {
        let history = ConversationHistory::new(vec![
            make_text_message("user", "hello"),
            make_parts_message(
                "user",
                vec![
                    ConversationMessageContentPart::Text {
                        text: "look at this".to_string(),
                    },
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "http://example.com/img.png".to_string(),
                        },
                    },
                ],
            ),
        ]);

        let urls = history.extract_image_urls();

        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].url, "http://example.com/img.png");
    }

    #[test]
    fn replace_images_with_marker_replaces_image_parts() {
        let history = ConversationHistory::new(vec![make_parts_message(
            "user",
            vec![
                ConversationMessageContentPart::Text {
                    text: "before".to_string(),
                },
                ConversationMessageContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "http://example.com/img.png".to_string(),
                    },
                },
                ConversationMessageContentPart::Text {
                    text: "after".to_string(),
                },
            ],
        )]);

        let marker = MediaMarker::new("[IMAGE]".to_string());
        let result = history.replace_images_with_marker(&marker);

        let parts = match &result.messages[0].content {
            ChatTemplateMessageContent::Parts(parts) => parts,
            ChatTemplateMessageContent::Text(_) => panic!("expected Parts variant"),
        };

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].text, "before");
        assert_eq!(parts[1].text, "[IMAGE]");
        assert_eq!(parts[2].text, "after");
    }

    #[test]
    fn replace_images_with_marker_preserves_text_messages() {
        let history = ConversationHistory::new(vec![make_text_message("assistant", "hello")]);

        let marker = MediaMarker::new("[IMAGE]".to_string());
        let result = history.replace_images_with_marker(&marker);

        assert_eq!(
            result.messages[0].content,
            ChatTemplateMessageContent::Text("hello".to_string())
        );
        assert_eq!(result.messages[0].role, "assistant");
    }
}
