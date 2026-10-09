use serde::Deserialize;

use crate::responses_function_output_part::ResponsesFunctionOutputPart;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ResponsesFunctionOutput {
    Text(String),
    Parts(Vec<ResponsesFunctionOutputPart>),
}

impl ResponsesFunctionOutput {
    #[must_use]
    pub fn into_text(self) -> String {
        match self {
            Self::Text(text) => text,
            Self::Parts(parts) => parts
                .into_iter()
                .map(|ResponsesFunctionOutputPart::InputText { text }| text)
                .collect::<String>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ResponsesFunctionOutput;
    use crate::responses_function_output_part::ResponsesFunctionOutputPart;

    #[test]
    fn text_output_returns_its_text() {
        assert_eq!(
            ResponsesFunctionOutput::Text("done".to_owned()).into_text(),
            "done"
        );
    }

    #[test]
    fn parts_output_concatenates_its_text() {
        let output = ResponsesFunctionOutput::Parts(vec![
            ResponsesFunctionOutputPart::InputText {
                text: "foo".to_owned(),
            },
            ResponsesFunctionOutputPart::InputText {
                text: "bar".to_owned(),
            },
        ]);

        assert_eq!(output.into_text(), "foobar");
    }
}
