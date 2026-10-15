use serde::Deserialize;
use serde::Deserializer;
use serde_json::Map;
use serde_json::Value;

fn criteria_or_none_described<'de, TDeserializer>(
    deserializer: TDeserializer,
) -> Result<Map<String, Value>, TDeserializer::Error>
where
    TDeserializer: Deserializer<'de>,
{
    Ok(Option::<Map<String, Value>>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "type")]
pub enum SystemOneQuestion {
    Choice {
        criteria: Map<String, Value>,
        #[serde(default)]
        instructions: Value,
    },
    Noul {
        #[serde(default, deserialize_with = "criteria_or_none_described")]
        criteria: Map<String, Value>,
        #[serde(default)]
        instructions: Value,
    },
    Score {
        criteria: Vec<Value>,
        #[serde(default)]
        instructions: Value,
    },
}
