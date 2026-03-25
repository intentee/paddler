use serde::Deserialize;
use serde::Serialize;

use crate::embedding::Embedding;
use crate::streamable_result::StreamableResult;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EmbeddingResult {
    Done,
    Embedding(Embedding),
    Error(String),
}

impl StreamableResult for EmbeddingResult {
    fn is_done(&self) -> bool {
        matches!(self, Self::Done | Self::Error(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding_normalization_method::EmbeddingNormalizationMethod;
    use crate::pooling_type::PoolingType;

    #[test]
    fn done_is_done() {
        assert!(EmbeddingResult::Done.is_done());
    }

    #[test]
    fn error_is_done() {
        assert!(EmbeddingResult::Error("fail".to_owned()).is_done());
    }

    #[test]
    fn embedding_is_not_done() {
        let result = EmbeddingResult::Embedding(Embedding {
            embedding: vec![1.0],
            normalization_method: EmbeddingNormalizationMethod::None,
            pooling_type: PoolingType::Mean,
            source_document_id: "doc".to_owned(),
        });

        assert!(!result.is_done());
    }
}
