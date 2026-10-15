use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::normalization::normalize_embedding::normalize_embedding;
use paddler_agent_embeddings::normalizes_embedding::NormalizesEmbedding;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;

pub const DIMENSIONS_TOO_MANY_FOR_RMS_NORM: usize = 1 << 16;

pub struct NormalizationOfAnEmbeddingTooLongForRmsNorm;

impl NormalizesEmbedding for NormalizationOfAnEmbeddingTooLongForRmsNorm {
    fn normalize(
        &self,
        _embedding: Vec<f32>,
        _normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<Vec<f32>, EmbeddingError> {
        normalize_embedding(
            vec![0.0; DIMENSIONS_TOO_MANY_FOR_RMS_NORM],
            &EmbeddingNormalizationMethod::RmsNorm { epsilon: 0.0 },
        )
    }
}
