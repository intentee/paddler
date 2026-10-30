use std::ops::Deref;
use std::sync::Arc;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::Error as _;
use serde_json::from_str;
use serde_json::value::RawValue;

#[derive(Clone, Debug)]
pub enum EmbeddingVector {
    Computed(Arc<[f32]>),
    Received {
        json: Arc<RawValue>,
        values: Arc<[f32]>,
    },
}

impl Deref for EmbeddingVector {
    type Target = [f32];

    fn deref(&self) -> &[f32] {
        match self {
            Self::Computed(values) | Self::Received { values, .. } => values,
        }
    }
}

impl From<Vec<f32>> for EmbeddingVector {
    fn from(values: Vec<f32>) -> Self {
        Self::Computed(values.into())
    }
}

impl PartialEq for EmbeddingVector {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Serialize for EmbeddingVector {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        match self {
            Self::Computed(values) => values.serialize(serializer),
            Self::Received { json, .. } => json.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for EmbeddingVector {
    fn deserialize<TDeserializer>(deserializer: TDeserializer) -> Result<Self, TDeserializer::Error>
    where
        TDeserializer: Deserializer<'de>,
    {
        let json = Box::<RawValue>::deserialize(deserializer)?;
        let values = from_str::<Vec<f32>>(json.get()).map_err(TDeserializer::Error::custom)?;

        Ok(Self::Received {
            json: Arc::from(json),
            values: values.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;
    use serde_json::to_string;

    use super::EmbeddingVector;

    #[test]
    fn a_received_vector_relays_the_json_it_arrived_as() {
        let received: EmbeddingVector = from_str("[0.25, 0.750]").unwrap();

        assert_eq!(
            [
                to_string(&received).unwrap(),
                to_string(&received.clone()).unwrap()
            ],
            ["[0.25, 0.750]", "[0.25, 0.750]"]
        );
    }

    #[test]
    fn refuses_a_vector_that_is_not_json() {
        assert_eq!(
            from_str::<EmbeddingVector>("[0.25,")
                .unwrap_err()
                .to_string(),
            "EOF while parsing a value at line 1 column 6"
        );
    }

    #[test]
    fn refuses_a_vector_of_values_that_are_not_numbers() {
        assert_eq!(
            from_str::<EmbeddingVector>(r#"["a"]"#)
                .unwrap_err()
                .to_string(),
            "invalid type: string \"a\", expected f32 at line 1 column 4"
        );
    }
}
