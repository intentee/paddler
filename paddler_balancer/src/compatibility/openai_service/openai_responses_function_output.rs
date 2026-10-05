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

#[cfg(test)]
mod tests {
    use super::OpenAIResponsesFunctionOutput;
    use crate::compatibility::openai_service::openai_responses_input_content_part::OpenAIResponsesInputContentPart;

    #[test]
    fn text_output_returns_its_text() {
        assert_eq!(
            OpenAIResponsesFunctionOutput::Text("done".to_owned()).into_text(),
            "done"
        );
    }

    #[test]
    fn parts_output_concatenates_text_and_drops_non_text() {
        let output = OpenAIResponsesFunctionOutput::Parts(vec![
            OpenAIResponsesInputContentPart::InputText {
                text: "foo".to_owned(),
            },
            OpenAIResponsesInputContentPart::InputImage { image_url: None },
            OpenAIResponsesInputContentPart::InputText {
                text: "bar".to_owned(),
            },
        ]);

        assert_eq!(output.into_text(), "foobar");
    }
}
