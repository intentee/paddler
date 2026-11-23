use std::ffi::NulError;
use std::num::TryFromIntError;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::error::decode_error::DecodeError;
use llama_cpp_bindings::error::embeddings_error::EmbeddingsError;
use llama_cpp_bindings::error::string_to_token_error::StringToTokenError;
use log::error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_messaging::embedding_result::EmbeddingResult;

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error(transparent)]
    AgentRuntime(#[from] AgentRuntimeError),

    #[error("failed to tokenize embedding input {source_document_id:?}")]
    InputTokenizationFailed {
        source_document_id: String,
        #[source]
        source: StringToTokenError,
    },

    #[error("failed to tokenize embedding input {source_document_id:?}")]
    InputContainsNul {
        source_document_id: String,
        #[source]
        source: NulError,
    },

    #[error("failed to tokenize embedding input {source_document_id:?}")]
    InputLengthUnrepresentable {
        source_document_id: String,
        #[source]
        source: TryFromIntError,
    },

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error("the scheduler is no longer accepting requests")]
    SchedulerUnavailable,

    #[error("failed to add an embedding input to the batch")]
    BatchAssemblyFailed(#[source] BatchAddError),

    #[error("the client stopped listening for embeddings")]
    ClientDisconnected(#[source] SendError<EmbeddingResult>),

    #[error("llama.cpp rejected the embedding batch")]
    DecodeFailed(#[source] DecodeError),

    #[error("{dimensions}-dimensional embedding is too long for RMS normalization")]
    EmbeddingTooLongForRmsNormalization {
        dimensions: usize,
        #[source]
        source: TryFromIntError,
    },

    #[error("failed to read the embedding of a batch sequence")]
    EmbeddingsUnavailable(#[source] EmbeddingsError),
}

impl EmbeddingError {
    #[must_use]
    pub fn for_input_tokenization_error(
        source_document_id: String,
        tokenization_error: StringToTokenError,
    ) -> Self {
        match tokenization_error {
            StringToTokenError::NulError(nul_error) => Self::InputContainsNul {
                source_document_id,
                source: nul_error,
            },
            StringToTokenError::CIntConversionError(conversion_error) => {
                Self::InputLengthUnrepresentable {
                    source_document_id,
                    source: conversion_error,
                }
            }
            StringToTokenError::CppString(_)
            | StringToTokenError::FfiContract(_)
            | StringToTokenError::FfiStatus(_)
            | StringToTokenError::LlamaCppOutOfMemory
            | StringToTokenError::NotEnoughMemory
            | StringToTokenError::Reported { .. } => Self::InputTokenizationFailed {
                source_document_id,
                source: tokenization_error,
            },
        }
    }

    pub fn report(
        self,
        agent_name: Option<&str>,
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
    ) {
        self.report_to_each(agent_name, [generated_embedding_tx]);
    }

    pub fn report_model_not_loaded(
        agent_name: Option<&str>,
        generated_embedding_tx: &mpsc::UnboundedSender<EmbeddingResult>,
    ) {
        Self::ModelNotLoaded.report(agent_name, generated_embedding_tx);
    }

    pub fn report_to_each<'sender>(
        self,
        agent_name: Option<&str>,
        generated_embedding_txs: impl IntoIterator<
            Item = &'sender mpsc::UnboundedSender<EmbeddingResult>,
        >,
    ) {
        let message = rejection_description(agent_name, &self);

        error!("{message}");

        let result = match self {
            Self::AgentRuntime(_) => EmbeddingResult::AgentRuntimeFailed(message),
            Self::BatchAssemblyFailed(_) => EmbeddingResult::BatchAssemblyFailed(message),
            Self::ClientDisconnected(_) => return,
            Self::DecodeFailed(_) => EmbeddingResult::DecodeFailed(message),
            Self::EmbeddingTooLongForRmsNormalization { .. } => {
                EmbeddingResult::EmbeddingTooLongForRmsNormalization(message)
            }
            Self::EmbeddingsUnavailable(_) => EmbeddingResult::EmbeddingsUnavailable(message),
            Self::InputContainsNul { .. }
            | Self::InputLengthUnrepresentable { .. }
            | Self::InputTokenizationFailed { .. } => {
                EmbeddingResult::InputTokenizationFailed(message)
            }
            Self::ModelNotLoaded => EmbeddingResult::ModelNotLoaded(message),
            Self::SchedulerUnavailable => EmbeddingResult::SchedulerUnavailable(message),
        };

        for generated_embedding_tx in generated_embedding_txs {
            send_result_or_warn(agent_name, generated_embedding_tx, result.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::mem::discriminant;

    use llama_cpp_bindings::batch_add_error::BatchAddError;
    use llama_cpp_bindings::error::decode_error::DecodeError;
    use llama_cpp_bindings::error::embeddings_error::EmbeddingsError;
    use llama_cpp_bindings::error::string_to_token_error::StringToTokenError;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::SendError;
    use tokio::sync::mpsc::error::TryRecvError;

    use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
    use paddler_messaging::embedding_result::EmbeddingResult;

    use super::EmbeddingError;

    #[test]
    fn every_embedding_failure_reports_its_own_embedding_result() {
        let description = String::new;
        let failures = [
            (
                EmbeddingError::AgentRuntime(AgentRuntimeError::AvailableParallelismUnknown(
                    io::Error::other("parallelism unknown"),
                )),
                EmbeddingResult::AgentRuntimeFailed(description()),
            ),
            (
                EmbeddingError::BatchAssemblyFailed(BatchAddError::NoSequenceIds),
                EmbeddingResult::BatchAssemblyFailed(description()),
            ),
            (
                EmbeddingError::DecodeFailed(DecodeError::NoKvCacheSlot),
                EmbeddingResult::DecodeFailed(description()),
            ),
            (
                EmbeddingError::EmbeddingTooLongForRmsNormalization {
                    dimensions: usize::MAX,
                    source: u16::try_from(usize::MAX).unwrap_err(),
                },
                EmbeddingResult::EmbeddingTooLongForRmsNormalization(description()),
            ),
            (
                EmbeddingError::EmbeddingsUnavailable(EmbeddingsError::NotEnabled),
                EmbeddingResult::EmbeddingsUnavailable(description()),
            ),
            (
                EmbeddingError::for_input_tokenization_error(
                    "document".to_owned(),
                    StringToTokenError::NotEnoughMemory,
                ),
                EmbeddingResult::InputTokenizationFailed(description()),
            ),
            (
                EmbeddingError::ModelNotLoaded,
                EmbeddingResult::ModelNotLoaded(description()),
            ),
            (
                EmbeddingError::SchedulerUnavailable,
                EmbeddingResult::SchedulerUnavailable(description()),
            ),
        ];

        for (embedding_error, expected_result) in failures {
            let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

            embedding_error.report(None, &generated_embedding_tx);

            assert_eq!(
                generated_embedding_rx.try_recv().as_ref().map(discriminant),
                Ok(discriminant(&expected_result))
            );
        }
    }

    #[test]
    fn a_document_too_long_for_llama_cpp_describes_the_conversion_failure_once() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

        EmbeddingError::for_input_tokenization_error(
            "document".to_owned(),
            StringToTokenError::CIntConversionError(u8::try_from(256_u16).unwrap_err()),
        )
        .report(Some("agent"), &generated_embedding_tx);

        assert_eq!(
            generated_embedding_rx.try_recv(),
            Ok(EmbeddingResult::InputTokenizationFailed(
                "agent: failed to tokenize embedding input \"document\": number too large to fit in target type"
                    .to_owned()
            ))
        );
    }

    #[test]
    fn sends_nothing_to_a_client_that_already_stopped_listening() {
        let (generated_embedding_tx, mut generated_embedding_rx) = mpsc::unbounded_channel();

        EmbeddingError::ClientDisconnected(SendError(EmbeddingResult::Done))
            .report(Some("agent"), &generated_embedding_tx);

        assert_eq!(generated_embedding_rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn tolerates_a_disconnected_client() {
        let (generated_embedding_tx, generated_embedding_rx) = mpsc::unbounded_channel();

        drop(generated_embedding_rx);

        EmbeddingError::SchedulerUnavailable.report(None, &generated_embedding_tx);

        assert!(generated_embedding_tx.is_closed());
    }

    #[test]
    fn reports_one_failure_to_every_request_it_affects() {
        let (first_embedding_tx, mut first_embedding_rx) = mpsc::unbounded_channel();
        let (second_embedding_tx, mut second_embedding_rx) = mpsc::unbounded_channel();

        EmbeddingError::DecodeFailed(DecodeError::NoKvCacheSlot)
            .report_to_each(None, [&first_embedding_tx, &second_embedding_tx]);

        let expected_result = Ok(EmbeddingResult::DecodeFailed(
            "llama.cpp rejected the embedding batch: no KV cache slot was available".to_owned(),
        ));

        assert_eq!(first_embedding_rx.try_recv(), expected_result);
        assert_eq!(second_embedding_rx.try_recv(), expected_result);
    }
}
