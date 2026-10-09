use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::error::ClearKvCacheSeqError;
use llama_cpp_bindings::error::CopyKvCacheSeqError;
use llama_cpp_bindings::error::DecodeError;
use llama_cpp_bindings::error::EmbeddingsError;
use llama_cpp_bindings::error::MetaValError;
use llama_cpp_bindings::error::StringToTokenError;
use llama_cpp_bindings::llama_token_attrs_from_int_error::LlamaTokenAttrsFromIntError;
use log::error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;

#[derive(Debug, thiserror::Error)]
pub enum DecisionError {
    #[error(transparent)]
    AgentRuntime(#[from] AgentRuntimeError),

    #[error("failed to add a decision token to the batch: {0}")]
    BatchAssemblyFailed(#[source] BatchAddError),

    #[error("the client stopped listening for decisions: {0}")]
    ClientDisconnected(#[source] SendError<DecisionResult>),

    #[error("llama.cpp rejected the decision batch: {0}")]
    DecodeFailed(#[source] DecodeError),

    #[error("unable to read the attributes of the pointer head delimiter {delimiter}: {source}")]
    DelimiterAttributesUnreadable {
        delimiter: String,
        #[source]
        source: LlamaTokenAttrsFromIntError,
    },

    #[error("unable to tokenize the pointer head delimiter {delimiter}: {source}")]
    DelimiterTokenizationFailed {
        delimiter: String,
        #[source]
        source: StringToTokenError,
    },

    #[error("unable to read the hidden state of a decision output: {0}")]
    HiddenStateUnavailable(#[source] EmbeddingsError),

    #[error("unable to expose the hidden states decisions read: {0}")]
    HiddenStatesUnsupported(#[source] EmbeddingsError),

    #[error("the agent serves {serving_inference_mode:?}, so it cannot decide")]
    InferenceModeMismatch {
        serving_inference_mode: InferenceMode,
    },

    #[error("failed to tokenize the decision input: {0}")]
    InputTokenizationFailed(#[source] StringToTokenError),

    #[error("failed to copy the decision state into its question lane: {0}")]
    KvCacheCopyFailed(#[source] CopyKvCacheSeqError),

    #[error("failed to remove a decision sequence from the KV cache: {0}")]
    KvCacheRemovalFailed(#[source] ClearKvCacheSeqError),

    #[error("unable to read the model architecture: {0}")]
    ModelArchitectureUnreadable(#[source] MetaValError),

    #[error("{architecture} models cannot serve decisions")]
    ModelArchitectureUnsupported { architecture: String },

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error("unable to load the pointer head: {0}")]
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
    pub fn report(
        self,
        agent_name: Option<&str>,
        decision_result_tx: &mpsc::UnboundedSender<DecisionResult>,
    ) {
        let message = rejection_description(agent_name, &self);

        error!("{message}");

        let result = match self {
            Self::AgentRuntime(_)
            | Self::DelimiterAttributesUnreadable { .. }
            | Self::DelimiterTokenizationFailed { .. }
            | Self::HiddenStatesUnsupported(_)
            | Self::ModelArchitectureUnreadable(_)
            | Self::ModelArchitectureUnsupported { .. }
            | Self::ModelNotLoaded
            | Self::PointerHeadCannotBeLoaded(_)
            | Self::PointerHeadIncompatibleWithModel { .. }
            | Self::SlotsInsufficient { .. } => DecisionResult::ModelNotLoaded(message),
            Self::BatchAssemblyFailed(_) => DecisionResult::BatchAssemblyFailed(message),
            Self::ClientDisconnected(_) => return,
            Self::DecodeFailed(_) => DecisionResult::DecodeFailed(message),
            Self::HiddenStateUnavailable(_) => DecisionResult::HiddenStateUnavailable(message),
            Self::InferenceModeMismatch { .. } => DecisionResult::InferenceModeMismatch(message),
            Self::InputTokenizationFailed(_) => DecisionResult::InputTokenizationFailed(message),
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
    use std::ffi::CString;
    use std::mem::discriminant;

    use llama_cpp_bindings::batch_add_error::BatchAddError;
    use llama_cpp_bindings::error::ClearKvCacheSeqError;
    use llama_cpp_bindings::error::CopyKvCacheSeqError;
    use llama_cpp_bindings::error::DecodeError;
    use llama_cpp_bindings::error::EmbeddingsError;
    use llama_cpp_bindings::error::StringToTokenError;
    use tokio::sync::mpsc;
    use tokio::sync::mpsc::error::SendError;

    use paddler_messaging::decision_result::DecisionResult;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;

    use super::DecisionError;

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
                DecisionError::BatchAssemblyFailed(BatchAddError::InsufficientSpace(1)),
                DecisionResult::BatchAssemblyFailed(description()),
            ),
            (
                DecisionError::DecodeFailed(DecodeError::NoKvCacheSlot),
                DecisionResult::DecodeFailed(description()),
            ),
            (
                DecisionError::HiddenStateUnavailable(EmbeddingsError::NextnEmbeddingsNotEnabled),
                DecisionResult::HiddenStateUnavailable(description()),
            ),
            (
                DecisionError::InferenceModeMismatch {
                    serving_inference_mode: InferenceMode::Embeddings,
                },
                DecisionResult::InferenceModeMismatch(description()),
            ),
            (
                DecisionError::InputTokenizationFailed(StringToTokenError::NulError(
                    CString::new("a \0 byte").unwrap_err(),
                )),
                DecisionResult::InputTokenizationFailed(description()),
            ),
            (
                DecisionError::KvCacheCopyFailed(CopyKvCacheSeqError::MemoryHandleUnavailable),
                DecisionResult::KvCacheCopyFailed(description()),
            ),
            (
                DecisionError::KvCacheRemovalFailed(
                    ClearKvCacheSeqError::PartialSequenceNotRemoved {
                        seq_id: 0,
                        p0: -1,
                        p1: -1,
                    },
                ),
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
    fn a_disconnected_client_is_not_sent_a_result() {
        assert_eq!(
            reported(DecisionError::ClientDisconnected(SendError(
                DecisionResult::SchedulerUnavailable(String::new())
            ))),
            None
        );
    }
}
