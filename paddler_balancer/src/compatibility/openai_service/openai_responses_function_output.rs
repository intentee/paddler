use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_input_content_part::OpenAIResponsesInputContentPart;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum OpenAIResponsesFunctionOutput {
    Text(String),
    Parts(Vec<OpenAIResponsesInputContentPart>),
}

impl OpenAIResponsesFunctionOutput {
    #[must_use]
    pub fn into_text(self) -> String {
        match self {
            Self::Text(text) => text,
            Self::Parts(parts) => parts
                .into_iter()
                .filter_map(|part| match part {
                    OpenAIResponsesInputContentPart::InputText { text } => Some(text),
                    OpenAIResponsesInputContentPart::InputImage { .. }
                    | OpenAIResponsesInputContentPart::Unsupported => None,
                })
                .collect::<String>(),
        }
    }
}
