use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChatTemplate {
    pub content: String,
}
