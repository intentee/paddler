use serde::Deserialize;
use serde::Serialize;

use crate::conversation_message_content::ConversationMessageContent;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationMessage {
    pub content: ConversationMessageContent,
    pub role: String,
}
