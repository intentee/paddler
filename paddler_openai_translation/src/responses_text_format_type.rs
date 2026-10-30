use serde::Deserialize;

#[derive(Deserialize)]
#[serde(variant_identifier)]
pub enum ResponsesTextFormatType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_schema")]
    JsonSchema,
}
