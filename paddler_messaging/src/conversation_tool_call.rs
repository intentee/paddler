use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationToolCall {
    pub arguments: Map<String, Value>,
    pub id: String,
    pub name: String,
}
