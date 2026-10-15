use serde::Deserialize;

#[derive(Deserialize)]
pub struct ResponsesFunctionCallItem {
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}
