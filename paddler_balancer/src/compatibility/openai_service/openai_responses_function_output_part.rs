use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIResponsesFunctionOutputPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
}
