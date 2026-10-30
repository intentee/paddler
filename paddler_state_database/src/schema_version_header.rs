use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct SchemaVersionHeader {
    pub version: Value,
}
