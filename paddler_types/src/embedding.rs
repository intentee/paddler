use anyhow::Result;
use anyhow::anyhow;
use serde::Deserialize;
use serde::Serialize;

use crate::embedding_normalization_method::EmbeddingNormalizationMethod;
use crate::normalization::l2;
use crate::normalization::rms_norm;
use crate::pooling_type::PoolingType;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Embedding {
    pub embedding: Vec<f32>,
    pub normalization_method: EmbeddingNormalizationMethod,
    pub pooling_type: PoolingType,
    pub source_document_id: String,
}

impl Embedding {
    pub fn normalize(self, normalization_method: &EmbeddingNormalizationMethod) -> Result<Self> {
        if !self
            .normalization_method
            .can_transform_to(normalization_method)
        {
            return Err(anyhow!(
                "Cannot transform from {:?} to {normalization_method:?}",
                self.normalization_method
            ));
        }

        if !self
            .normalization_method
            .needs_transformation_to(normalization_method)
        {
            return Ok(self);
        }

        Ok(Self {
            embedding: match normalization_method {
                EmbeddingNormalizationMethod::None => self.embedding,
                EmbeddingNormalizationMethod::L2 => l2(&self.embedding),
                EmbeddingNormalizationMethod::RmsNorm { epsilon } => {
                    rms_norm(&self.embedding, *epsilon)
                }
            },
            normalization_method: normalization_method.clone(),
            pooling_type: self.pooling_type.clone(),
            source_document_id: self.source_document_id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedding(values: Vec<f32>, method: EmbeddingNormalizationMethod) -> Embedding {
        Embedding {
            embedding: values,
            normalization_method: method,
            pooling_type: PoolingType::Mean,
            source_document_id: "test".to_string(),
        }
    }

    #[test]
    fn test_normalize_from_none_to_l2() {
        let embedding = make_embedding(vec![3.0, 4.0], EmbeddingNormalizationMethod::None);
        let result = embedding
            .normalize(&EmbeddingNormalizationMethod::L2)
            .unwrap();

        assert_eq!(result.embedding, vec![0.6, 0.8]);
        assert!(matches!(
            result.normalization_method,
            EmbeddingNormalizationMethod::L2
        ));
    }

    #[test]
    fn test_normalize_from_none_to_rms_norm() {
        let embedding =
            make_embedding(vec![2.0, 2.0, 2.0, 2.0], EmbeddingNormalizationMethod::None);
        let result = embedding
            .normalize(&EmbeddingNormalizationMethod::RmsNorm { epsilon: 0.0 })
            .unwrap();

        for val in &result.embedding {
            assert!((val - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_normalize_none_to_none_is_noop() {
        let embedding = make_embedding(vec![1.0, 2.0, 3.0], EmbeddingNormalizationMethod::None);
        let result = embedding
            .normalize(&EmbeddingNormalizationMethod::None)
            .unwrap();

        assert_eq!(result.embedding, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_normalize_rejects_l2_to_rms_norm() {
        let embedding = make_embedding(vec![0.6, 0.8], EmbeddingNormalizationMethod::L2);
        let result = embedding.normalize(&EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 });

        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_rejects_l2_to_none() {
        let embedding = make_embedding(vec![0.6, 0.8], EmbeddingNormalizationMethod::L2);
        let result = embedding.normalize(&EmbeddingNormalizationMethod::None);

        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_rejects_rms_norm_to_l2() {
        let embedding = make_embedding(
            vec![1.0, 1.0],
            EmbeddingNormalizationMethod::RmsNorm { epsilon: 1e-6 },
        );
        let result = embedding.normalize(&EmbeddingNormalizationMethod::L2);

        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_preserves_metadata() {
        let embedding = Embedding {
            embedding: vec![3.0, 4.0],
            normalization_method: EmbeddingNormalizationMethod::None,
            pooling_type: PoolingType::Cls,
            source_document_id: "doc-42".to_string(),
        };
        let result = embedding
            .normalize(&EmbeddingNormalizationMethod::L2)
            .unwrap();

        assert!(matches!(result.pooling_type, PoolingType::Cls));
        assert_eq!(result.source_document_id, "doc-42");
    }
}
