use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_function_output_part::OpenAIResponsesFunctionOutputPart;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum OpenAIResponsesFunctionOutput {
    Text(String),
    Parts(Vec<OpenAIResponsesFunctionOutputPart>),
}

impl OpenAIResponsesFunctionOutput {
    #[must_use]
    pub fn into_text(self) -> String {
        match self {
            Self::Text(text) => text,
            Self::Parts(parts) => parts
                .into_iter()
                .map(|OpenAIResponsesFunctionOutputPart::InputText { text }| text)
                .collect::<String>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OpenAIResponsesFunctionOutput;
    use crate::compatibility::openai_service::openai_responses_function_output_part::OpenAIResponsesFunctionOutputPart;

    #[test]
    fn text_output_returns_its_text() {
        assert_eq!(
            OpenAIResponsesFunctionOutput::Text("done".to_owned()).into_text(),
            "done"
        );
    }

    #[test]
    fn parts_output_concatenates_its_text() {
        let output = OpenAIResponsesFunctionOutput::Parts(vec![
            OpenAIResponsesFunctionOutputPart::InputText {
                text: "foo".to_owned(),
            },
            OpenAIResponsesFunctionOutputPart::InputText {
                text: "bar".to_owned(),
            },
        ]);

        assert_eq!(output.into_text(), "foobar");
    }
}
