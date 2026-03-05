use serde::Serialize;

use crate::conversation_message::ConversationMessage;
use crate::conversation_message_content::ConversationMessageContent;
use crate::conversation_message_content_part::ConversationMessageContentPart;
use crate::image_url::ImageUrl;

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct ConversationMessageCollection {
    messages: Vec<ConversationMessage>,
}

impl ConversationMessageCollection {
    pub fn new(messages: Vec<ConversationMessage>) -> Self {
        Self { messages }
    }

    pub fn extract_image_urls(&self) -> Vec<&ImageUrl> {
        self.messages
            .iter()
            .flat_map(|message| message.content.image_urls())
            .collect()
    }

    pub fn to_text_only(&self, media_marker: &str) -> Self {
        Self {
            messages: self
                .messages
                .iter()
                .map(|message| ConversationMessage {
                    content: match &message.content {
                        ConversationMessageContent::Text(text) => {
                            ConversationMessageContent::Text(text.clone())
                        }
                        ConversationMessageContent::Parts(parts) => {
                            ConversationMessageContent::Parts(
                                parts
                                    .iter()
                                    .map(|part| match part {
                                        ConversationMessageContentPart::Text { text } => {
                                            ConversationMessageContentPart::Text {
                                                text: text.clone(),
                                            }
                                        }
                                        ConversationMessageContentPart::ImageUrl { .. } => {
                                            ConversationMessageContentPart::Text {
                                                text: media_marker.to_string(),
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
