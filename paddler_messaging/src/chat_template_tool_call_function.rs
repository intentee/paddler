use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTemplateToolCallFunction {
    pub arguments: Map<String, Value>,
    pub name: String,
}
