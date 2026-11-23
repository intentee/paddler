use std::ffi::NulError;
use std::num::TryFromIntError;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::error::decode_error::DecodeError;
use llama_cpp_bindings::error::embeddings_error::EmbeddingsError;
use llama_cpp_bindings::error::memory_seq_cp_error::MemorySeqCpError;
use llama_cpp_bindings::error::memory_seq_rm_error::MemorySeqRmError;
use llama_cpp_bindings::error::string_to_token_error::StringToTokenError;
use llama_cpp_bindings::error::token_attr_error::TokenAttrError;
use log::error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;

use crate::reports_decision_outcome::ReportsDecisionOutcome;

#[derive(Debug, thiserror::Error)]
pub enum DecisionError {
    #[error(transparent)]
    AgentRuntime(#[from] AgentRuntimeError),

    #[error("failed to add a decision token to the batch")]
    BatchAssemblyFailed(#[source] BatchAddError),

    #[error("the client stopped listening for decisions")]
    ClientDisconnected(#[source] SendError<DecisionResult>),

    #[error("no pointer head is configured, so the agent serves no decisions")]
    DecisionsNotServed,

    #[error("llama.cpp rejected the decision batch")]
    DecodeFailed(#[source] DecodeError),

    #[error("unable to read the attributes of the pointer head delimiter {delimiter}")]
    DelimiterAttributesUnreadable {
        delimiter: String,
        #[source]
        source: TokenAttrError,
    },

    #[error("unable to tokenize the pointer head delimiter {delimiter}")]
    DelimiterTokenizationFailed {
        delimiter: String,
        #[source]
        source: StringToTokenError,
    },

    #[error("unable to tokenize the pointer head delimiter {delimiter}")]
    DelimiterContainsNul {
        delimiter: String,
        #[source]
        source: NulError,
    },

    #[error("unable to tokenize the pointer head delimiter {delimiter}")]
    DelimiterLengthUnrepresentable {
        delimiter: String,
        #[source]
        source: TryFromIntError,
    },

    #[error("unable to read the hidden state of a decision output")]
    HiddenStateUnavailable(#[source] EmbeddingsError),

    #[error("failed to tokenize the decision input")]
    InputTokenizationFailed(#[source] StringToTokenError),

    #[error("failed to tokenize the decision input")]
    InputContainsNul(#[source] NulError),

    #[error("failed to tokenize the decision input")]
    InputLengthUnrepresentable(#[source] TryFromIntError),

    #[error("failed to copy the decision state into its question lane")]
    KvCacheCopyFailed(#[source] MemorySeqCpError),

    #[error("failed to remove a decision sequence from the KV cache")]
    KvCacheRemovalFailed(#[source] MemorySeqRmError),

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error("unable to load the pointer head")]
    PointerHeadCannotBeLoaded(#[source] PointerHeadError),

    #[error("the pointer head does not fit the model: {incompatibility:?}")]
    PointerHeadIncompatibleWithModel {
        incompatibility: PointerHeadIncompatibility,
    },

    #[error(
        "the decision needs {} tokens in the cache, more than the {} the context holds",
        details.required_tokens,
        details.context_size
    )]
    RequestExceedsContext { details: OversizedDecisionDetails },

    #[error("the scheduler is no longer accepting requests")]
    SchedulerUnavailable,

    #[error("decisions need {required_slots} slots, the agent runs {desired_slots}")]
    SlotsInsufficient {
        desired_slots: u16,
        required_slots: u16,
    },
}

impl DecisionError {
    #[must_use]
    pub fn for_delimiter_tokenization_error(
        delimiter: String,
        tokenization_error: StringToTokenError,
    ) -> Self {
        match tokenization_error {
            StringToTokenError::NulError(nul_error) => Self::DelimiterContainsNul {
                delimiter,
                source: nul_error,
            },
            StringToTokenError::CIntConversionError(conversion_error) => {
                Self::DelimiterLengthUnrepresentable {
                    delimiter,
                    source: conversion_error,
                }
            }
            StringToTokenError::CppString(_)
            | StringToTokenError::FfiContract(_)
            | StringToTokenError::FfiStatus(_)
            | StringToTokenError::LlamaCppOutOfMemory
            | StringToTokenError::NotEnoughMemory
            | StringToTokenError::Reported { .. } => Self::DelimiterTokenizationFailed {
                delimiter,
                source: tokenization_error,
            },
        }
    }

    #[must_use]
    pub fn for_input_tokenization_error(tokenization_error: StringToTokenError) -> Self {
        match tokenization_error {
            StringToTokenError::NulError(nul_error) => Self::InputContainsNul(nul_error),
            StringToTokenError::CIntConversionError(conversion_error) => {
                Self::InputLengthUnrepresentable(conversion_error)
            }
            StringToTokenError::CppString(_)
            | StringToTokenError::FfiContract(_)
            | StringToTokenError::FfiStatus(_)
            | StringToTokenError::LlamaCppOutOfMemory
            | StringToTokenError::NotEnoughMemory
            | StringToTokenError::Reported { .. } => {
                Self::InputTokenizationFailed(tokenization_error)
            }
        }
    }

    pub fn report_decisions_not_served(
        agent_name: Option<&str>,
        decision_result_tx: &mpsc::UnboundedSender<DecisionResult>,
    ) {
        Self::DecisionsNotServed.report(agent_name, decision_result_tx);
    }

    pub fn report_model_not_loaded(
        agent_name: Option<&str>,
        decision_result_tx: &mpsc::UnboundedSender<DecisionResult>,
    ) {
        Self::ModelNotLoaded.report(agent_name, decision_result_tx);
    }
}

impl ReportsDecisionOutcome for DecisionError {
    fn report(
        self,
        agent_name: Option<&str>,
        decision_result_tx: &mpsc::UnboundedSender<DecisionResult>,
    ) {
        let message = rejection_description(agent_name, &self);

        error!("{message}");

        let result = match self {
            Self::AgentRuntime(_)
            | Self::DelimiterAttributesUnreadable { .. }
            | Self::DelimiterContainsNul { .. }
            | Self::DelimiterLengthUnrepresentable { .. }
            | Self::DelimiterTokenizationFailed { .. }
            | Self::ModelNotLoaded
            | Self::PointerHeadCannotBeLoaded(_)
            | Self::PointerHeadIncompatibleWithModel { .. }
            | Self::SlotsInsufficient { .. } => DecisionResult::ModelNotLoaded(message),
            Self::BatchAssemblyFailed(_) => DecisionResult::BatchAssemblyFailed(message),
            Self::ClientDisconnected(_) => return,
            Self::DecisionsNotServed => DecisionResult::DecisionsNotServed(message),
            Self::DecodeFailed(_) => DecisionResult::DecodeFailed(message),
            Self::HiddenStateUnavailable(_) => DecisionResult::HiddenStateUnavailable(message),
            Self::InputContainsNul(_)
            | Self::InputLengthUnrepresentable(_)
            | Self::InputTokenizationFailed(_) => DecisionResult::InputTokenizationFailed(message),
            Self::KvCacheCopyFailed(_) => DecisionResult::KvCacheCopyFailed(message),
            Self::KvCacheRemovalFailed(_) => DecisionResult::KvCacheRemovalFailed(message),
            Self::RequestExceedsContext { details } => {
                DecisionResult::RequestExceedsContext(details)
            }
            Self::SchedulerUnavailable => DecisionResult::SchedulerUnavailable(message),
        };

        send_result_or_warn(agent_name, decision_result_tx, result);
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::batch_add_error::BatchAddError;
    use llama_cpp_bindings::error::decode_error::DecodeError;
    use llama_cpp_bindings::error::embeddings_error::EmbeddingsError;
    use llama_cpp_bindings::error::memory_seq_cp_error::MemorySeqCpError;
    use llama_cpp_bindings::error::memory_seq_rm_error::MemorySeqRmError;
    use llama_cpp_bindings::error::string_to_token_error::StringToTokenError;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::SendError;

    use paddler_messaging::decision_result::DecisionResult;
    use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;

    use super::DecisionError;
    use crate::reports_decision_outcome::ReportsDecisionOutcome as _;

    fn reported(decision_error: DecisionError) -> Option<DecisionResult> {
        let (decision_result_tx, mut decision_result_rx) = mpsc::unbounded_channel();

        decision_error.report(None, &decision_result_tx);

        decision_result_rx.try_recv().ok()
    }

    #[test]
    fn every_request_failure_reports_its_own_decision_result() {
        let description = String::new;
        let oversized = OversizedDecisionDetails {
            context_size: 256,
            required_tokens: 900,
        };
        let failures = [
            (
                DecisionError::BatchAssemblyFailed(BatchAddError::BatchFull { n_batch: 1 }),
                DecisionResult::BatchAssemblyFailed(description()),
            ),
            (
                DecisionError::DecisionsNotServed,
                DecisionResult::DecisionsNotServed(description()),
            ),
            (
                DecisionError::DecodeFailed(DecodeError::NoKvCacheSlot),
                DecisionResult::DecodeFailed(description()),
            ),
            (
                DecisionError::HiddenStateUnavailable(EmbeddingsError::TokenNotAnOutput {
                    token_index: 0,
                }),
                DecisionResult::HiddenStateUnavailable(description()),
            ),
            (
                DecisionError::for_input_tokenization_error(StringToTokenError::NotEnoughMemory),
                DecisionResult::InputTokenizationFailed(description()),
            ),
            (
                DecisionError::KvCacheCopyFailed(MemorySeqCpError::MemoryHandleUnavailable),
                DecisionResult::KvCacheCopyFailed(description()),
            ),
            (
                DecisionError::KvCacheRemovalFailed(MemorySeqRmError::PartialSequenceNotRemoved {
                    seq_id: 0,
                    p0: -1,
                    p1: -1,
                }),
                DecisionResult::KvCacheRemovalFailed(description()),
            ),
            (
                DecisionError::ModelNotLoaded,
                DecisionResult::ModelNotLoaded(description()),
            ),
            (
                DecisionError::RequestExceedsContext {
                    details: oversized.clone(),
                },
                DecisionResult::RequestExceedsContext(oversized),
            ),
            (
                DecisionError::SchedulerUnavailable,
                DecisionResult::SchedulerUnavailable(description()),
            ),
        ];

        for (decision_error, expected_result) in failures {
            assert_eq!(
                reported(decision_error).as_ref().map(discriminant),
                Some(discriminant(&expected_result))
            );
        }
    }

    #[test]
    fn a_decision_input_too_long_for_llama_cpp_describes_the_conversion_failure_once() {
        assert_eq!(
            reported(DecisionError::for_input_tokenization_error(
                StringToTokenError::CIntConversionError(u8::try_from(256_u16).unwrap_err())
            )),
            Some(DecisionResult::InputTokenizationFailed(
                "failed to tokenize the decision input: number too large to fit in target type"
                    .to_owned()
            ))
        );
    }

    #[test]
    fn a_delimiter_too_long_for_llama_cpp_describes_the_conversion_failure_once() {
        assert_eq!(
            reported(DecisionError::for_delimiter_tokenization_error(
                "<decide>".to_owned(),
                StringToTokenError::CIntConversionError(u8::try_from(256_u16).unwrap_err())
            )),
            Some(DecisionResult::ModelNotLoaded(
                "unable to tokenize the pointer head delimiter <decide>: number too large to fit in target type"
                    .to_owned()
            ))
        );
    }

    #[test]
    fn a_delimiter_llama_cpp_cannot_tokenize_is_described_through_its_cause() {
        assert_eq!(
            reported(DecisionError::for_delimiter_tokenization_error(
                "<decide>".to_owned(),
                StringToTokenError::NotEnoughMemory
            )),
            Some(DecisionResult::ModelNotLoaded(
                "unable to tokenize the pointer head delimiter <decide>: not enough memory"
                    .to_owned()
            ))
        );
    }

    #[test]
    fn a_disconnected_client_is_not_sent_a_result() {
        assert_eq!(
            reported(DecisionError::ClientDisconnected(SendError(
                DecisionResult::SchedulerUnavailable(String::new())
            ))),
            None
        );
    }
}
