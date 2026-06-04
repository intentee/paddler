use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIResponsesTextFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_schema")]
    JsonSchema { schema: Value },
    #[serde(other)]
    Unsupported,
}
