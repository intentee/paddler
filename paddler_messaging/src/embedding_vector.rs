use std::sync::Arc;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde_json::Error as JsonError;
use serde_json::from_str;
use serde_json::value::RawValue;

#[derive(Clone, Debug)]
pub enum EmbeddingVector {
    Computed(Arc<[f32]>),
    Received(Arc<RawValue>),
}

impl EmbeddingVector {
    /// # Errors
    ///
    /// Returns the [`JsonError`] of a received vector that is not an array of numbers.
    pub fn values(&self) -> Result<Arc<[f32]>, JsonError> {
        match self {
            Self::Computed(values) => Ok(values.clone()),
            Self::Received(json) => from_str::<Vec<f32>>(json.get()).map(Arc::from),
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
        match (self, other) {
            (Self::Computed(own_values), Self::Computed(other_values)) => {
                own_values == other_values
            }
            (Self::Received(own_json), Self::Received(other_json)) => {
                own_json.get() == other_json.get()
            }
            (Self::Computed(computed_values), Self::Received(received_json))
            | (Self::Received(received_json), Self::Computed(computed_values)) => {
                from_str::<Vec<f32>>(received_json.get())
                    .is_ok_and(|received_values| *received_values == **computed_values)
            }
        }
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
            Self::Received(json) => json.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for EmbeddingVector {
    fn deserialize<TDeserializer>(deserializer: TDeserializer) -> Result<Self, TDeserializer::Error>
    where
        TDeserializer: Deserializer<'de>,
    {
        Box::<RawValue>::deserialize(deserializer).map(|json| Self::Received(Arc::from(json)))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::error::Category;
    use serde_json::from_str;
    use serde_json::to_string;

    use super::EmbeddingVector;

    #[test]
    fn a_received_vector_relays_the_json_it_arrived_as() {
        let received: EmbeddingVector =
            from_str("[0.25, 0.750]").expect("a JSON array must be received");

        assert_eq!(
            to_string(&received).expect("a received vector must serialize"),
            "[0.25, 0.750]"
        );
    }

    #[test]
    fn a_received_vector_reads_its_values_from_its_json() {
        let received: EmbeddingVector =
            from_str("[0.25, 0.750]").expect("a JSON array must be received");

        assert_eq!(
            *received
                .values()
                .expect("an array of numbers must have values"),
            [0.25, 0.75]
        );
    }

    #[test]
    fn a_computed_vector_reads_the_values_it_was_computed_with() {
        assert_eq!(
            *EmbeddingVector::from(vec![0.25, 0.75])
                .values()
                .expect("a computed vector must have values"),
            [0.25, 0.75]
        );
    }

    #[test]
    fn refuses_a_vector_that_is_not_json() {
        assert!(matches!(
            from_str::<EmbeddingVector>("[0.25,"),
            Err(deserialization_error) if deserialization_error.classify() == Category::Eof
                && deserialization_error.line() == 1
                && deserialization_error.column() == 6
        ));
    }

    #[test]
    fn a_received_vector_of_values_that_are_not_numbers_has_no_values() {
        let received: EmbeddingVector =
            from_str(r#"["a"]"#).expect("a JSON array must be received");

        assert!(matches!(
            received.values(),
            Err(value_error) if value_error.classify() == Category::Data
        ));
    }

    #[test]
    fn received_vectors_are_equal_when_their_json_is_equal() {
        let received: EmbeddingVector =
            from_str("[0.25, 0.75]").expect("a JSON array must be received");
        let relayed: EmbeddingVector =
            from_str("[0.25, 0.75]").expect("a JSON array must be received");

        assert_eq!(received, relayed);
    }

    #[test]
    fn a_received_vector_equals_the_computed_vector_of_the_same_values() {
        let received: EmbeddingVector =
            from_str("[0.25, 0.750]").expect("a JSON array must be received");

        assert_eq!(received, EmbeddingVector::from(vec![0.25, 0.75]));
        assert_eq!(EmbeddingVector::from(vec![0.25, 0.75]), received);
        assert_ne!(EmbeddingVector::from(vec![0.5]), received);
    }
}
