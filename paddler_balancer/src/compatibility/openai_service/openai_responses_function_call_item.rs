use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenAIResponsesFunctionCallItem {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}
