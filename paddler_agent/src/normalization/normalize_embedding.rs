use anyhow::Result;
use anyhow::anyhow;

use paddler_messaging::embedding::Embedding;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;

use crate::normalization::l2;
use crate::normalization::rms_norm;

pub fn normalize_embedding(
    embedding: Embedding,
    normalization_method: &EmbeddingNormalizationMethod,
) -> Result<Embedding> {
    if !embedding
        .normalization_method
        .can_transform_to(normalization_method)
    {
        return Err(anyhow!(
            "Cannot transform from {:?} to {normalization_method:?}",
            embedding.normalization_method
        ));
    }

    if !embedding
        .normalization_method
        .needs_transformation_to(normalization_method)
    {
        return Ok(embedding);
    }

    let normalized = match normalization_method {
        EmbeddingNormalizationMethod::None => embedding.embedding,
        EmbeddingNormalizationMethod::L2 => l2(&embedding.embedding),
        EmbeddingNormalizationMethod::RmsNorm { epsilon } => {
            rms_norm(&embedding.embedding, *epsilon)?
        }
    };

    Ok(Embedding {
        embedding: normalized,
        normalization_method: normalization_method.clone(),
        pooling_type: embedding.pooling_type,
        source_document_id: embedding.source_document_id,
    })
}

#[cfg(test)]
mod tests {
    use paddler_messaging::embedding::Embedding;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::pooling_type::PoolingType;

    use super::normalize_embedding;

    fn make_embedding(values: Vec<f32>, method: EmbeddingNormalizationMethod) -> Embedding {
        Embedding {
            embedding: values,
            normalization_method: method,
            pooling_type: PoolingType::Mean,
            source_document_id: "test".to_owned(),
        }
    }

    #[test]
    fn normalize_from_none_to_l2() {
        let embedding = make_embedding(vec![3.0, 4.0], EmbeddingNormalizationMethod::None);
        let result = normalize_embedding(embedding, &EmbeddingNormalizationMethod::L2).unwrap();

        assert_eq!(result.embedding, vec![0.6, 0.8]);
        assert!(
            !result
                .normalization_method
                .needs_transformation_to(&EmbeddingNormalizationMethod::L2)
        );
    }

    #[test]
    fn normalize_from_none_to_rms_norm() {
        let embedding =
            make_embedding(vec![2.0, 2.0, 2.0, 2.0], EmbeddingNormalizationMethod::None);
        let result = normalize_embedding(
            embedding,
            &EmbeddingNormalizationMethod::RmsNorm { epsilon: 0.0 },
        )
        .unwrap();

        for value in &result.embedding {
            assert!((value - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn normalize_none_to_none_is_noop() {
        let embedding = make_embedding(vec![1.0, 2.0, 3.0], EmbeddingNormalizationMethod::None);
        let result = normalize_embedding(embedding, &EmbeddingNormalizationMethod::None).unwrap();

        assert_eq!(result.embedding, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn normalize_rejects_l2_to_rms_norm() {
        let embedding = make_embedding(vec![0.6, 0.8], EmbeddingNormalizationMethod::L2);
        let result = normalize_embedding(
            embedding,
            &EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 },
        );

        assert!(result.is_err());
    }

    #[test]
    fn normalize_rejects_l2_to_none() {
        let embedding = make_embedding(vec![0.6, 0.8], EmbeddingNormalizationMethod::L2);
        let result = normalize_embedding(embedding, &EmbeddingNormalizationMethod::None);

        assert!(result.is_err());
    }

    #[test]
    fn normalize_rejects_rms_norm_to_l2() {
        let embedding = make_embedding(
            vec![1.0, 1.0],
            EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 },
        );
        let result = normalize_embedding(embedding, &EmbeddingNormalizationMethod::L2);

        assert!(result.is_err());
    }

    #[test]
    fn normalize_to_rms_norm_propagates_oversized_embedding_error() {
        let oversized_length = usize::from(u16::MAX) + 1;
        let embedding = make_embedding(
            vec![1.0; oversized_length],
            EmbeddingNormalizationMethod::None,
        );
        let result = normalize_embedding(
            embedding,
            &EmbeddingNormalizationMethod::RmsNorm { epsilon: 0.0 },
        );

        assert!(result.is_err());
    }

    #[test]
    fn normalize_preserves_metadata() {
        let embedding = Embedding {
            embedding: vec![3.0, 4.0],
            normalization_method: EmbeddingNormalizationMethod::None,
            pooling_type: PoolingType::Cls,
            source_document_id: "doc-42".to_owned(),
        };
        let result = normalize_embedding(embedding, &EmbeddingNormalizationMethod::L2).unwrap();

        assert_eq!(result.pooling_type, PoolingType::Cls);
        assert_eq!(result.source_document_id, "doc-42");
    }
}
