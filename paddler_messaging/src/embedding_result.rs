use serde::Deserialize;
use serde::Serialize;

use crate::embedding::Embedding;
use crate::embedding_document_exceeding_sequence_context_details::EmbeddingDocumentExceedingSequenceContextDetails;
use crate::embedding_document_tokenized_to_nothing_details::EmbeddingDocumentTokenizedToNothingDetails;
use crate::embedding_final_result::EmbeddingFinalResult;
use crate::embedding_partial_result::EmbeddingPartialResult;
use crate::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
use crate::result_step::ResultStep;
use crate::streamable_result::StreamableResult;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EmbeddingResult {
    AgentRuntimeFailed(String),
    BatchAssemblyFailed(String),
    DecodeFailed(String),
    DocumentExceedsBatchSize(OversizedEmbeddingDocumentDetails),
    DocumentExceedsSequenceContext(EmbeddingDocumentExceedingSequenceContextDetails),
    DocumentTokenizedToNothing(EmbeddingDocumentTokenizedToNothingDetails),
    Done,
    Embedding(Embedding),
    EmbeddingTooLongForRmsNormalization(String),
    EmbeddingsUnavailable(String),
    InputTokenizationFailed(String),
    ModelNotLoaded(String),
    NoEmbeddingsProduced,
    SchedulerUnavailable(String),
}

impl From<EmbeddingFinalResult> for EmbeddingResult {
    fn from(final_result: EmbeddingFinalResult) -> Self {
        match final_result {
            EmbeddingFinalResult::AgentRuntimeFailed(detail) => Self::AgentRuntimeFailed(detail),
            EmbeddingFinalResult::BatchAssemblyFailed(detail) => Self::BatchAssemblyFailed(detail),
            EmbeddingFinalResult::DecodeFailed(detail) => Self::DecodeFailed(detail),
            EmbeddingFinalResult::Done => Self::Done,
            EmbeddingFinalResult::EmbeddingTooLongForRmsNormalization(detail) => {
                Self::EmbeddingTooLongForRmsNormalization(detail)
            }
            EmbeddingFinalResult::EmbeddingsUnavailable(detail) => {
                Self::EmbeddingsUnavailable(detail)
            }
            EmbeddingFinalResult::InputTokenizationFailed(detail) => {
                Self::InputTokenizationFailed(detail)
            }
            EmbeddingFinalResult::ModelNotLoaded(detail) => Self::ModelNotLoaded(detail),
            EmbeddingFinalResult::NoEmbeddingsProduced => Self::NoEmbeddingsProduced,
            EmbeddingFinalResult::SchedulerUnavailable(detail) => {
                Self::SchedulerUnavailable(detail)
            }
        }
    }
}

impl From<EmbeddingPartialResult> for EmbeddingResult {
    fn from(partial_result: EmbeddingPartialResult) -> Self {
        match partial_result {
            EmbeddingPartialResult::DocumentExceedsBatchSize(details) => {
                Self::DocumentExceedsBatchSize(details)
            }
            EmbeddingPartialResult::DocumentExceedsSequenceContext(details) => {
                Self::DocumentExceedsSequenceContext(details)
            }
            EmbeddingPartialResult::DocumentTokenizedToNothing(details) => {
                Self::DocumentTokenizedToNothing(details)
            }
            EmbeddingPartialResult::Embedding(embedding) => Self::Embedding(embedding),
        }
    }
}

impl StreamableResult for EmbeddingResult {
    type Final = EmbeddingFinalResult;
    type Partial = EmbeddingPartialResult;

    fn into_step(self) -> ResultStep<EmbeddingPartialResult, EmbeddingFinalResult> {
        match self {
            Self::AgentRuntimeFailed(detail) => {
                ResultStep::Final(EmbeddingFinalResult::AgentRuntimeFailed(detail))
            }
            Self::BatchAssemblyFailed(detail) => {
                ResultStep::Final(EmbeddingFinalResult::BatchAssemblyFailed(detail))
            }
            Self::DecodeFailed(detail) => {
                ResultStep::Final(EmbeddingFinalResult::DecodeFailed(detail))
            }
            Self::DocumentExceedsBatchSize(details) => {
                ResultStep::Partial(EmbeddingPartialResult::DocumentExceedsBatchSize(details))
            }
            Self::DocumentExceedsSequenceContext(details) => ResultStep::Partial(
                EmbeddingPartialResult::DocumentExceedsSequenceContext(details),
            ),
            Self::DocumentTokenizedToNothing(details) => {
                ResultStep::Partial(EmbeddingPartialResult::DocumentTokenizedToNothing(details))
            }
            Self::Done => ResultStep::Final(EmbeddingFinalResult::Done),
            Self::Embedding(embedding) => {
                ResultStep::Partial(EmbeddingPartialResult::Embedding(embedding))
            }
            Self::EmbeddingTooLongForRmsNormalization(detail) => ResultStep::Final(
                EmbeddingFinalResult::EmbeddingTooLongForRmsNormalization(detail),
            ),
            Self::EmbeddingsUnavailable(detail) => {
                ResultStep::Final(EmbeddingFinalResult::EmbeddingsUnavailable(detail))
            }
            Self::InputTokenizationFailed(detail) => {
                ResultStep::Final(EmbeddingFinalResult::InputTokenizationFailed(detail))
            }
            Self::ModelNotLoaded(detail) => {
                ResultStep::Final(EmbeddingFinalResult::ModelNotLoaded(detail))
            }
            Self::NoEmbeddingsProduced => {
                ResultStep::Final(EmbeddingFinalResult::NoEmbeddingsProduced)
            }
            Self::SchedulerUnavailable(detail) => {
                ResultStep::Final(EmbeddingFinalResult::SchedulerUnavailable(detail))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_inference_parameters::pooling_type::PoolingType;

    use super::EmbeddingResult;
    use crate::embedding::Embedding;
    use crate::embedding_document_exceeding_sequence_context_details::EmbeddingDocumentExceedingSequenceContextDetails;
    use crate::embedding_document_tokenized_to_nothing_details::EmbeddingDocumentTokenizedToNothingDetails;
    use crate::embedding_final_result::EmbeddingFinalResult;
    use crate::embedding_normalization_method::EmbeddingNormalizationMethod;
    use crate::embedding_partial_result::EmbeddingPartialResult;
    use crate::embedding_vector::EmbeddingVector;
    use crate::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
    use crate::result_step::ResultStep;
    use crate::streamable_result::StreamableResult as _;

    fn step_round_trip(embedding_result: EmbeddingResult) -> EmbeddingResult {
        match embedding_result.into_step() {
            ResultStep::Final(final_result) => final_result.into(),
            ResultStep::Partial(partial_result) => partial_result.into(),
        }
    }

    #[test]
    fn every_terminal_result_ends_the_stream_as_its_final_result() {
        for (terminal_result, expected_final_result) in [
            (
                EmbeddingResult::AgentRuntimeFailed("runtime".to_owned()),
                EmbeddingFinalResult::AgentRuntimeFailed("runtime".to_owned()),
            ),
            (
                EmbeddingResult::BatchAssemblyFailed("batch".to_owned()),
                EmbeddingFinalResult::BatchAssemblyFailed("batch".to_owned()),
            ),
            (
                EmbeddingResult::DecodeFailed("decode".to_owned()),
                EmbeddingFinalResult::DecodeFailed("decode".to_owned()),
            ),
            (EmbeddingResult::Done, EmbeddingFinalResult::Done),
            (
                EmbeddingResult::EmbeddingTooLongForRmsNormalization("rms".to_owned()),
                EmbeddingFinalResult::EmbeddingTooLongForRmsNormalization("rms".to_owned()),
            ),
            (
                EmbeddingResult::EmbeddingsUnavailable("unavailable".to_owned()),
                EmbeddingFinalResult::EmbeddingsUnavailable("unavailable".to_owned()),
            ),
            (
                EmbeddingResult::InputTokenizationFailed("tokenization".to_owned()),
                EmbeddingFinalResult::InputTokenizationFailed("tokenization".to_owned()),
            ),
            (
                EmbeddingResult::ModelNotLoaded("no model".to_owned()),
                EmbeddingFinalResult::ModelNotLoaded("no model".to_owned()),
            ),
            (
                EmbeddingResult::NoEmbeddingsProduced,
                EmbeddingFinalResult::NoEmbeddingsProduced,
            ),
            (
                EmbeddingResult::SchedulerUnavailable("stopped".to_owned()),
                EmbeddingFinalResult::SchedulerUnavailable("stopped".to_owned()),
            ),
        ] {
            assert_eq!(step_round_trip(terminal_result.clone()), terminal_result);
            assert_eq!(
                terminal_result.into_step(),
                ResultStep::Final(expected_final_result)
            );
        }
    }

    #[test]
    fn every_per_document_result_keeps_the_stream_open() {
        let embedding = Embedding {
            embedding: EmbeddingVector::from(vec![1.0]),
            normalization_method: EmbeddingNormalizationMethod::None,
            pooling_type: PoolingType::Mean,
            source_document_id: "doc".to_owned(),
        };
        let oversized_document = OversizedEmbeddingDocumentDetails {
            document_tokens: 4096,
            n_batch: 2048,
            source_document_id: "huge".to_owned(),
        };
        let document_exceeding_sequence_context =
            EmbeddingDocumentExceedingSequenceContextDetails {
                document_tokens: 9000,
                sequence_context_size: 8192,
                source_document_id: "long".to_owned(),
            };
        let document_tokenized_to_nothing = EmbeddingDocumentTokenizedToNothingDetails {
            source_document_id: "empty".to_owned(),
        };

        for (per_document_result, expected_partial_result) in [
            (
                EmbeddingResult::DocumentExceedsBatchSize(oversized_document.clone()),
                EmbeddingPartialResult::DocumentExceedsBatchSize(oversized_document),
            ),
            (
                EmbeddingResult::DocumentExceedsSequenceContext(
                    document_exceeding_sequence_context.clone(),
                ),
                EmbeddingPartialResult::DocumentExceedsSequenceContext(
                    document_exceeding_sequence_context,
                ),
            ),
            (
                EmbeddingResult::DocumentTokenizedToNothing(document_tokenized_to_nothing.clone()),
                EmbeddingPartialResult::DocumentTokenizedToNothing(document_tokenized_to_nothing),
            ),
            (
                EmbeddingResult::Embedding(embedding.clone()),
                EmbeddingPartialResult::Embedding(embedding),
            ),
        ] {
            assert_eq!(
                step_round_trip(per_document_result.clone()),
                per_document_result
            );
            assert_eq!(
                per_document_result.into_step(),
                ResultStep::Partial(expected_partial_result)
            );
        }
    }
}
