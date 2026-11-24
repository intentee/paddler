use serde::Deserialize;
use serde::Serialize;

use crate::conversation_message::ConversationMessage;

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
}
