use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;

use crate::embedding_error::EmbeddingError;

pub trait NormalizesEmbedding {
    /// # Errors
    ///
    /// Returns [`EmbeddingError::EmbeddingTooLongForRmsNormalization`] when the embedding cannot be normalized with the requested method.
    fn normalize(
        &self,
        embedding: Vec<f32>,
        normalization_method: &EmbeddingNormalizationMethod,
    ) -> Result<Vec<f32>, EmbeddingError>;
}
