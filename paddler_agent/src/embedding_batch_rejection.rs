use std::num::TryFromIntError;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::error::DecodeError;
use llama_cpp_bindings::error::EmbeddingsError;
use llama_cpp_bindings::error::StringToTokenError;
use log::error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use paddler_messaging::embedding_result::EmbeddingResult;

use crate::rejection_description::rejection_description;
use crate::send_result_or_warn::send_result_or_warn;

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingBatchRejection {
    #[error("embeddings are not enabled")]
    EmbeddingsDisabled,

    #[error("failed to tokenize embedding input {source_document_id:?}: {source}")]
    InputTokenizationFailed {
        source_document_id: String,
        #[source]
        source: StringToTokenError,
    },

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error("the scheduler is no longer accepting requests")]
    SchedulerUnavailable,

    #[error("failed to add an embedding input to the batch: {0}")]
    BatchAssemblyFailed(#[source] BatchAddError),

    #[error("the client stopped listening for embeddings: {0}")]
    ClientDisconnected(#[source] SendError<EmbeddingResult>),

    #[error("llama.cpp rejected the embedding batch: {0}")]
    DecodeFailed(#[source] DecodeError),

    #[error("{dimensions}-dimensional embedding is too long for RMS normalization: {source}")]
    EmbeddingTooLongForRmsNormalization {
        dimensions: usize,
        #[source]
        source: TryFromIntError,
    },

    #[error("failed to read the embedding of a batch sequence: {0}")]
    EmbeddingsUnavailable(#[source] EmbeddingsError),

    #[error("embedding batch sequence index does not fit in i32: {0}")]
    SequenceIndexOutOfRange(#[source] TryFromIntError),
}

impl EmbeddingBatchRejection {
    pub fn report(
        self,
        agent_name: Option<&str>,
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
    ) {
        let message = rejection_description(agent_name, &self);

        error!("{message}");

        let result = match self {
            Self::EmbeddingsDisabled => EmbeddingResult::EmbeddingsDisabled,
            Self::ModelNotLoaded => EmbeddingResult::ModelNotLoaded(message),
            Self::InputTokenizationFailed { .. }
            | Self::SchedulerUnavailable
            | Self::BatchAssemblyFailed(_)
            | Self::ClientDisconnected(_)
            | Self::DecodeFailed(_)
            | Self::EmbeddingTooLongForRmsNormalization { .. }
            | Self::EmbeddingsUnavailable(_)
            | Self::SequenceIndexOutOfRange(_) => EmbeddingResult::Error(message),
        };

        send_result_or_warn(agent_name, generated_embedding_tx, result);
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use paddler_messaging::embedding_result::EmbeddingResult;

    use super::EmbeddingBatchRejection;

    #[test]
    fn reports_disabled_embeddings() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

        EmbeddingBatchRejection::EmbeddingsDisabled.report(Some("agent"), &generated_embedding_tx);

        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::EmbeddingsDisabled)
        );
    }

    #[test]
    fn reports_other_rejections_as_errors_naming_the_cause() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

        EmbeddingBatchRejection::SchedulerUnavailable
            .report(Some("agent"), &generated_embedding_tx);

        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::Error(
                "agent: the scheduler is no longer accepting requests".to_owned()
            ))
        );
    }

    #[test]
    fn tolerates_a_disconnected_client() {
        let (generated_embedding_tx, generated_embedding_rx) = mpsc::unbounded_channel();

        drop(generated_embedding_rx);

        EmbeddingBatchRejection::EmbeddingsDisabled.report(None, &generated_embedding_tx);

        assert!(generated_embedding_tx.is_closed());
    }
}
