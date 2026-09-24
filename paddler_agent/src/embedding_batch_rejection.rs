use std::num::TryFromIntError;

use llama_cpp_bindings::error::StringToTokenError;
use log::error;
use paddler_messaging::embedding_result::EmbeddingResult;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

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

    #[error("embedding size does not fit in u32: {0}")]
    SizeOutOfRange(#[source] TryFromIntError),

    #[error("the scheduler is no longer accepting requests")]
    SchedulerUnavailable,

    #[error("the client disconnected before the embedding batch was prepared: {0}")]
    ClientDisconnected(#[source] SendError<EmbeddingResult>),
}

impl EmbeddingBatchRejection {
    pub fn report(
        self,
        agent_name: Option<&str>,
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
    ) {
        let message = format!("{agent_name:?}: {self}");

        error!("{message}");

        let result = match self {
            Self::EmbeddingsDisabled => EmbeddingResult::EmbeddingsDisabled,
            Self::InputTokenizationFailed { .. }
            | Self::SizeOutOfRange(_)
            | Self::SchedulerUnavailable
            | Self::ClientDisconnected(_) => EmbeddingResult::Error(message),
        };

        if generated_embedding_tx.send(result).is_err() {
            error!(
                "{agent_name:?}: failed to send embedding rejection to client (receiver dropped)"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::embedding_result::EmbeddingResult;
    use tokio::sync::mpsc;

    use super::EmbeddingBatchRejection;

    #[test]
    fn reports_disabled_embeddings() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

        EmbeddingBatchRejection::EmbeddingsDisabled.report(Some("agent"), &generated_embedding_tx);

        assert!(matches!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::EmbeddingsDisabled)
        ));
    }

    #[test]
    fn reports_other_rejections_as_errors_naming_the_cause() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

        EmbeddingBatchRejection::SchedulerUnavailable
            .report(Some("agent"), &generated_embedding_tx);

        assert!(matches!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::Error(message))
                if message == "Some(\"agent\"): the scheduler is no longer accepting requests"
        ));
    }

    #[test]
    fn tolerates_a_disconnected_client() {
        let (generated_embedding_tx, generated_embedding_rx) = mpsc::unbounded_channel();

        drop(generated_embedding_rx);

        EmbeddingBatchRejection::EmbeddingsDisabled.report(None, &generated_embedding_tx);

        assert!(generated_embedding_tx.is_closed());
    }
}
