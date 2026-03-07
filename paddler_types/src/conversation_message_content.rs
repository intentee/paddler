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
    pub fn text_content(&self) -> String {
        match self {
            ConversationMessageContent::Text(text) => text.clone(),
            ConversationMessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ConversationMessageContentPart::Text { text } => Some(text.as_str()),
                    ConversationMessageContentPart::ImageUrl { .. } => None,
                })
                .collect::<Vec<&str>>()
                .join(""),
        }
    }

    pub fn image_urls(&self) -> Vec<&ImageUrl> {
        match self {
            ConversationMessageContent::Text(_) => vec![],
            ConversationMessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ConversationMessageContentPart::ImageUrl { image_url } => Some(image_url),
                    ConversationMessageContentPart::Text { .. } => None,
                })
                .collect(),
        }
    }
}
