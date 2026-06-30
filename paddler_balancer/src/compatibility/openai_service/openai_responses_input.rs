use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_input_item::OpenAIResponsesInputItem;

pub enum OpenAIResponsesInput {
    Text(String),
    Items(Vec<OpenAIResponsesInputItem>),
}

impl Default for OpenAIResponsesInput {
    fn default() -> Self {
        Self::Items(Vec::new())
    }
}

impl<'de> Deserialize<'de> for OpenAIResponsesInput {
    fn deserialize<TDeserializer>(deserializer: TDeserializer) -> Result<Self, TDeserializer::Error>
    where
        TDeserializer: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TextOrItems {
            Text(String),
            Items(Vec<OpenAIResponsesInputItem>),
        }

        Ok(match TextOrItems::deserialize(deserializer)? {
            TextOrItems::Text(text) => Self::Text(text),
            TextOrItems::Items(items) => Self::Items(items),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OpenAIResponsesInput;

    #[test]
    fn a_string_input_is_accepted_as_text() {
        let input: OpenAIResponsesInput =
            serde_json::from_value(json!("hello")).expect("a string is a valid responses input");

        assert!(matches!(input, OpenAIResponsesInput::Text(text) if text == "hello"));
    }

    #[test]
    fn an_array_input_is_accepted_as_items() {
        let input: OpenAIResponsesInput =
            serde_json::from_value(json!([{ "role": "user", "content": "hi" }]))
                .expect("an array is a valid responses input");

        assert!(matches!(input, OpenAIResponsesInput::Items(items) if items.len() == 1));
    }

    #[test]
    fn a_non_string_non_array_input_is_rejected() {
        assert!(serde_json::from_value::<OpenAIResponsesInput>(json!(42)).is_err());
    }

    #[test]
    fn the_default_input_is_empty_items() {
        assert!(matches!(
            OpenAIResponsesInput::default(),
            OpenAIResponsesInput::Items(items) if items.is_empty()
        ));
    }
}
