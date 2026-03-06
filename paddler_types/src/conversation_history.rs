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
