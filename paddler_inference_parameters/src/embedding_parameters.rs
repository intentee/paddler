use std::num::NonZeroUsize;

use serde::Deserialize;
use serde::Serialize;

use crate::invalid_inference_parameters::InvalidInferenceParameters;
use crate::pooling_type::PoolingType;
use crate::raw_embedding_parameters::RawEmbeddingParameters;

const DEFAULT_EMBEDDING_BATCH_SIZE: NonZeroUsize = NonZeroUsize::new(256).unwrap();

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "RawEmbeddingParameters")]
pub struct EmbeddingParameters {
    pub embedding_batch_size: NonZeroUsize,
    pub pooling_type: PoolingType,
}

impl Default for EmbeddingParameters {
    fn default() -> Self {
        Self {
            embedding_batch_size: DEFAULT_EMBEDDING_BATCH_SIZE,
            pooling_type: PoolingType::Last,
        }
    }
}

impl TryFrom<RawEmbeddingParameters> for EmbeddingParameters {
    type Error = InvalidInferenceParameters;

    fn try_from(
        RawEmbeddingParameters {
            embedding_batch_size,
            pooling_type,
        }: RawEmbeddingParameters,
    ) -> Result<Self, Self::Error> {
        if pooling_type == PoolingType::None {
            return Err(InvalidInferenceParameters::PoolingProducesNoEmbeddings);
        }

        Ok(Self {
            embedding_batch_size,
            pooling_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;

    use super::EmbeddingParameters;
    use crate::invalid_inference_parameters::InvalidInferenceParameters;

    #[test]
    fn deserialization_rejects_pooling_that_produces_no_embeddings() {
        assert_eq!(
            from_value::<EmbeddingParameters>(json!({
                "embedding_batch_size": 256,
                "pooling_type": "None",
            }))
            .err()
            .as_ref()
            .map(ToString::to_string),
            Some(InvalidInferenceParameters::PoolingProducesNoEmbeddings.to_string())
        );
    }
}
