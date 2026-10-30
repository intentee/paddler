use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;

use crate::embedding_error::EmbeddingError;
use crate::normalization::normalize_embedding::normalize_embedding;
use crate::normalizes_embedding::NormalizesEmbedding;

pub struct EmbeddingNormalization;

impl NormalizesEmbedding for EmbeddingNormalization {
    fn normalize(
        &self,
        embedding: Vec<f32>,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<Vec<f32>, EmbeddingError> {
        normalize_embedding(embedding, normalization_method)
    }
}
