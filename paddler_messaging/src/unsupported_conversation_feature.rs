use serde::Deserialize;
use serde::Serialize;

use crate::conversation_feature::ConversationFeature;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedConversationFeature {
    pub description: String,
    pub feature: ConversationFeature,
}
