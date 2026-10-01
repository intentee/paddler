use serde::Deserialize;
use serde::Serialize;

use crate::chat_template_conversation::ChatTemplateConversation;
use crate::chat_template_message::ChatTemplateMessage;
use crate::chat_template_message_content::ChatTemplateMessageContent;
use crate::chat_template_message_content_part::ChatTemplateMessageContentPart;
use crate::conversation_message::ConversationMessage;
use crate::conversation_message_content::ConversationMessageContent;
use crate::conversation_message_content_part::ConversationMessageContentPart;
use crate::media_marker::MediaMarker;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ConversationHistory {
    pub messages: Vec<ConversationMessage>,
}

impl ConversationHistory {
    #[must_use]
    pub const fn new(messages: Vec<ConversationMessage>) -> Self {
        Self { messages }
    }

    #[must_use]
    pub fn into_chat_template_conversation(
        self,
        media_marker: &MediaMarker,
    ) -> ChatTemplateConversation {
        let mut image_urls = Vec::new();
        let messages = self
            .messages
            .into_iter()
            .map(
                |ConversationMessage { content, role }| ChatTemplateMessage {
                    content: match content {
                        ConversationMessageContent::Text(text) => {
                            ChatTemplateMessageContent::Text(text)
                        }
                        ConversationMessageContent::Parts(parts) => {
                            ChatTemplateMessageContent::Parts(
                                parts
                                    .into_iter()
                                    .map(|part| ChatTemplateMessageContentPart {
                                        content_type: "text",
                                        text: match part {
                                            ConversationMessageContentPart::Text { text } => text,
                                            ConversationMessageContentPart::ImageUrl {
                                                image_url,
                                            } => {
                                                image_urls.push(image_url);

                                                media_marker.marker.clone()
                                            }
                                        },
                                    })
                                    .collect(),
                            )
                        }
                    },
                    role,
                },
            )
            .collect();

        ChatTemplateConversation {
            image_urls,
            messages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConversationHistory;
    use crate::chat_template_conversation::ChatTemplateConversation;
    use crate::chat_template_message::ChatTemplateMessage;
    use crate::chat_template_message_content::ChatTemplateMessageContent;
    use crate::chat_template_message_content_part::ChatTemplateMessageContentPart;
    use crate::conversation_message::ConversationMessage;
    use crate::conversation_message_content::ConversationMessageContent;
    use crate::conversation_message_content_part::ConversationMessageContentPart;
    use crate::image_url::ImageUrl;
    use crate::media_marker::MediaMarker;

    fn make_text_message(role: &str, text: &str) -> ConversationMessage {
        ConversationMessage {
            content: ConversationMessageContent::Text(text.to_owned()),
            role: role.to_owned(),
        }
    }

    fn make_parts_message(
        role: &str,
        parts: Vec<ConversationMessageContentPart>,
    ) -> ConversationMessage {
        ConversationMessage {
            content: ConversationMessageContent::Parts(parts),
            role: role.to_owned(),
        }
    }

    #[test]
    fn collects_image_urls_and_replaces_images_with_the_media_marker() {
        let history = ConversationHistory::new(vec![
            make_text_message("assistant", "hello"),
            make_parts_message(
                "user",
                vec![
                    ConversationMessageContentPart::Text {
                        text: "before".to_owned(),
                    },
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "http://example.com/img.png".to_owned(),
                        },
                    },
                    ConversationMessageContentPart::Text {
                        text: "after".to_owned(),
                    },
                ],
            ),
        ]);

        assert_eq!(
            history.into_chat_template_conversation(&MediaMarker::new("[IMAGE]".to_owned())),
            ChatTemplateConversation {
                image_urls: vec![ImageUrl {
                    url: "http://example.com/img.png".to_owned(),
                }],
                messages: vec![
                    ChatTemplateMessage {
                        content: ChatTemplateMessageContent::Text("hello".to_owned()),
                        role: "assistant".to_owned(),
                    },
                    ChatTemplateMessage {
                        content: ChatTemplateMessageContent::Parts(vec![
                            ChatTemplateMessageContentPart {
                                content_type: "text",
                                text: "before".to_owned(),
                            },
                            ChatTemplateMessageContentPart {
                                content_type: "text",
                                text: "[IMAGE]".to_owned(),
                            },
                            ChatTemplateMessageContentPart {
                                content_type: "text",
                                text: "after".to_owned(),
                            },
                        ]),
                        role: "user".to_owned(),
                    },
                ],
            }
        );
    }
}
