use serde::Deserialize;
use serde::Deserializer;

use crate::responses_input_item::ResponsesInputItem;

pub enum ResponsesInput {
    Text(String),
    Items(Vec<ResponsesInputItem>),
}

impl Default for ResponsesInput {
    fn default() -> Self {
        Self::Items(Vec::new())
    }
}

impl<'deserializer> Deserialize<'deserializer> for ResponsesInput {
    fn deserialize<TDeserializer>(deserializer: TDeserializer) -> Result<Self, TDeserializer::Error>
    where
        TDeserializer: Deserializer<'deserializer>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TextOrItems {
            Text(String),
            Items(Vec<ResponsesInputItem>),
        }

        Ok(match TextOrItems::deserialize(deserializer)? {
            TextOrItems::Text(text) => Self::Text(text),
            TextOrItems::Items(items) => Self::Items(items),
        })
    }
}
