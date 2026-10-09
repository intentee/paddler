use serde::Deserialize;
use serde::Serialize;

use crate::embedding::Embedding;
use crate::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
use crate::streamable_result::StreamableResult;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EmbeddingResult {
    AgentRuntimeFailed(String),
    BatchAssemblyFailed(String),
    DecodeFailed(String),
    DocumentExceedsBatchSize(OversizedEmbeddingDocumentDetails),
    Done,
    Embedding(Embedding),
    EmbeddingTooLongForRmsNormalization(String),
    EmbeddingsUnavailable(String),
    InferenceModeMismatch(String),
    InputTokenizationFailed(String),
    ModelNotLoaded(String),
    NoEmbeddingsProduced,
    SchedulerUnavailable(String),
}

impl StreamableResult for EmbeddingResult {
    fn is_done(&self) -> bool {
        matches!(
            self,
            Self::AgentRuntimeFailed(_)
                | Self::BatchAssemblyFailed(_)
                | Self::DecodeFailed(_)
                | Self::Done
                | Self::EmbeddingTooLongForRmsNormalization(_)
                | Self::EmbeddingsUnavailable(_)
                | Self::InferenceModeMismatch(_)
                | Self::InputTokenizationFailed(_)
                | Self::ModelNotLoaded(_)
                | Self::NoEmbeddingsProduced
                | Self::SchedulerUnavailable(_),
        )
    }
}

#[cfg(test)]
mod tests {
    use paddler_inference_parameters::pooling_type::PoolingType;

    use super::EmbeddingResult;
    use crate::embedding::Embedding;
    use crate::embedding_normalization_method::EmbeddingNormalizationMethod;
    use crate::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
    use crate::streamable_result::StreamableResult;

    #[test]
    fn every_terminal_result_ends_the_stream() {
        for terminal_result in [
            EmbeddingResult::AgentRuntimeFailed("failure".to_owned()),
            EmbeddingResult::BatchAssemblyFailed("failure".to_owned()),
            EmbeddingResult::DecodeFailed("failure".to_owned()),
            EmbeddingResult::Done,
            EmbeddingResult::EmbeddingTooLongForRmsNormalization("failure".to_owned()),
            EmbeddingResult::EmbeddingsUnavailable("failure".to_owned()),
            EmbeddingResult::InferenceModeMismatch("failure".to_owned()),
            EmbeddingResult::InputTokenizationFailed("failure".to_owned()),
            EmbeddingResult::ModelNotLoaded("failure".to_owned()),
            EmbeddingResult::NoEmbeddingsProduced,
            EmbeddingResult::SchedulerUnavailable("failure".to_owned()),
        ] {
            assert!(
                terminal_result.is_done(),
                "{terminal_result:?} must end the stream"
            );
        }
    }

    #[test]
    fn document_exceeds_batch_size_is_not_done() {
        let result = EmbeddingResult::DocumentExceedsBatchSize(OversizedEmbeddingDocumentDetails {
            document_tokens: 4096,
            n_batch: 2048,
            source_document_id: "huge".to_owned(),
        });

        assert!(!result.is_done());
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
