use serde::Deserialize;

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub enum ResponsesTextFormatField {
    Schema,
    Type,
    #[serde(other)]
    Unrecognized,
}
