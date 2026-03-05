use serde::Deserialize;
use serde::Serialize;

use crate::chat_template_message::ChatTemplateMessage;
use crate::chat_template_messages::ChatTemplateMessages;
use crate::conversation_message::ConversationMessage;
use crate::conversation_message_content::ConversationMessageContent;
use crate::conversation_message_content_part::ConversationMessageContentPart;
use crate::image_url::ImageUrl;
use crate::media_marker::MediaMarker;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
        ChatTemplateMessages {
            messages: self
                .messages
                .iter()
                .map(|message| ChatTemplateMessage {
                    content: match &message.content {
                        ConversationMessageContent::Text(text) => text.clone(),
                        ConversationMessageContent::Parts(parts) => parts
                            .iter()
                            .map(|part| match part {
                                ConversationMessageContentPart::Text { text } => text.clone(),
                                ConversationMessageContentPart::ImageUrl { .. } => {
                                    media_marker.to_string()
                                }
                            })
                            .collect::<Vec<String>>()
                            .join(""),
                    },
                    role: message.role.clone(),
                })
                .collect(),
        }
    }
}
