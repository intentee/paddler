use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;

use crate::embedding_error::EmbeddingError;
use crate::normalization::l2::l2;
use crate::normalization::rms_norm::rms_norm;

pub fn normalize_embedding(
    mut embedding: Vec<f32>,
    normalization_method: &EmbeddingNormalizationMethod,
) -> Result<Vec<f32>, EmbeddingError> {
    match normalization_method {
        EmbeddingNormalizationMethod::None => {}
        EmbeddingNormalizationMethod::L2 => l2(&mut embedding),
        EmbeddingNormalizationMethod::RmsNorm { epsilon } => {
            rms_norm(&mut embedding, *epsilon).map_err(|source| {
                EmbeddingError::EmbeddingTooLongForRmsNormalization {
                    dimensions: embedding.len(),
                    source,
                }
            })?;
        }
    }

    Ok(embedding)
}

#[cfg(test)]
mod tests {
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;

    use super::normalize_embedding;
    use crate::embedding_error::EmbeddingError;

    #[test]
    fn scales_the_embedding_to_unit_length_with_l2() {
        let normalized =
            normalize_embedding(vec![3.0, 4.0], &EmbeddingNormalizationMethod::L2).unwrap();

        assert_eq!(normalized, vec![0.6, 0.8]);
    }

    #[test]
    fn divides_the_embedding_by_its_root_mean_square_with_rms_norm() {
        let normalized = normalize_embedding(
            vec![2.0, 2.0, 2.0, 2.0],
            &EmbeddingNormalizationMethod::RmsNorm { epsilon: 0.0 },
        )
        .unwrap();

        assert_eq!(normalized, vec![1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn leaves_the_embedding_unchanged_without_normalization() {
        let normalized =
            normalize_embedding(vec![1.0, 2.0, 3.0], &EmbeddingNormalizationMethod::None).unwrap();

        assert_eq!(normalized, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn rejects_an_embedding_too_long_for_rms_norm() {
        let oversized_length = usize::from(u16::MAX) + 1;
        let result = normalize_embedding(
            vec![1.0; oversized_length],
            &EmbeddingNormalizationMethod::RmsNorm { epsilon: 0.0 },
        );

        assert!(matches!(
            result,
            Err(EmbeddingError::EmbeddingTooLongForRmsNormalization {
                dimensions,
                ..
            }) if dimensions == oversized_length
        ));
    }
}
