use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ResponsesFunctionOutputPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
}
