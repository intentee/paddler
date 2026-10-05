use serde::Deserialize;
use serde::Deserializer;

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

impl<'deserializer> Deserialize<'deserializer> for OpenAIResponsesInput {
    fn deserialize<TDeserializer>(deserializer: TDeserializer) -> Result<Self, TDeserializer::Error>
    where
        TDeserializer: Deserializer<'deserializer>,
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
