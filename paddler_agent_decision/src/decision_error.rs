use llama_cpp_context::context_error::ContextError;
use llama_cpp_context_memory::context_memory_error::ContextMemoryError;
use llama_cpp_decision::decision_error::DecisionError as LlamaDecisionError;
use llama_cpp_mtmd::mtmd_error::MtmdError;
use log::error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_runtime::send_result_or_warn::send_result_or_warn;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;

use crate::reports_decision_outcome::ReportsDecisionOutcome;

#[derive(Debug, thiserror::Error)]
pub enum DecisionError {
    #[error(transparent)]
    AgentRuntime(#[from] AgentRuntimeError),

    #[error("llama.cpp cannot read the answers of a decision prompt")]
    AnswersUnreadable(#[source] LlamaDecisionError),

    #[error("failed to add a decision token to the batch")]
    BatchAssemblyFailed(#[source] ContextError),

    #[error("the client stopped listening for decisions")]
    ClientDisconnected(#[source] Box<SendError<DecisionResult>>),

    #[error("the agent serves no decisions with the loaded model")]
    DecisionsNotServed,

    #[error("llama.cpp rejected the decision batch")]
    DecodeFailed(#[source] ContextError),

    #[error("failed to copy the shared decision prompt into its question lane")]
    KvCacheCopyFailed(#[source] ContextMemoryError),

    #[error("failed to remove a decision sequence from the KV cache")]
    KvCacheRemovalFailed(#[source] ContextMemoryError),

    #[error("llama.cpp cannot evaluate the media of a decision prompt")]
    MediaEvaluationFailed(#[source] MtmdError),

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error(
        "a decision prompt of {prompt_tokens} tokens must be decoded at once, but a batch holds {batch_tokens} tokens"
    )]
    PromptExceedsBatch {
        batch_tokens: usize,
        prompt_tokens: usize,
    },

    #[error(
        "the answer of a decision prompt is read from its last {read_tokens} tokens, but a batch holds {batch_tokens} tokens"
    )]
    PromptReadSpanExceedsBatch {
        batch_tokens: usize,
        read_tokens: usize,
    },

    #[error("llama.cpp cannot lay out the prompts of the decision")]
    PromptsRejected(#[source] LlamaDecisionError),

    #[error(
        "the decision needs {} tokens in the cache, more than the {} the context holds",
        details.required_tokens,
        details.context_size
    )]
    RequestExceedsContext { details: OversizedDecisionDetails },

    #[error("llama.cpp rejected the decision request")]
    RequestRejected(#[source] LlamaDecisionError),

    #[error("the scheduler is no longer accepting requests")]
    SchedulerUnavailable,

    #[error("the agent has {desired_slots} slots, but decisions need at least {required_slots}")]
    SlotsInsufficient {
        desired_slots: u16,
        required_slots: u16,
    },
}

impl DecisionError {
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
            Self::AgentRuntime(_) | Self::ModelNotLoaded | Self::SlotsInsufficient { .. } => {
                DecisionResult::ModelNotLoaded(message)
            }
            Self::AnswersUnreadable(_) => DecisionResult::AnswersUnreadable(message),
            Self::BatchAssemblyFailed(_) => DecisionResult::BatchAssemblyFailed(message),
            Self::ClientDisconnected(_) => return,
            Self::DecisionsNotServed => DecisionResult::DecisionsNotServed(message),
            Self::DecodeFailed(_) => DecisionResult::DecodeFailed(message),
            Self::KvCacheCopyFailed(_) => DecisionResult::KvCacheCopyFailed(message),
            Self::KvCacheRemovalFailed(_) => DecisionResult::KvCacheRemovalFailed(message),
            Self::MediaEvaluationFailed(_) => DecisionResult::MediaEvaluationFailed(message),
            Self::PromptExceedsBatch { .. }
            | Self::PromptReadSpanExceedsBatch { .. }
            | Self::PromptsRejected(_)
            | Self::RequestRejected(_) => DecisionResult::RequestRejected(message),
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

    use llama_cpp_context::context_error::ContextError;
    use llama_cpp_context_memory::context_memory_error::ContextMemoryError;
    use llama_cpp_decision::decision_error::DecisionError as LlamaDecisionError;
    use llama_cpp_mtmd::mtmd_error::MtmdError;
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
                DecisionError::AnswersUnreadable(LlamaDecisionError::AnswersNotEnoughMemory),
                DecisionResult::AnswersUnreadable(description()),
            ),
            (
                DecisionError::BatchAssemblyFailed(ContextError::BatchFull { n_batch: 1 }),
                DecisionResult::BatchAssemblyFailed(description()),
            ),
            (
                DecisionError::DecisionsNotServed,
                DecisionResult::DecisionsNotServed(description()),
            ),
            (
                DecisionError::DecodeFailed(ContextError::DecodeNoKvCacheSlot),
                DecisionResult::DecodeFailed(description()),
            ),
            (
                DecisionError::KvCacheCopyFailed(ContextMemoryError::SeqCpMemoryHandleUnavailable),
                DecisionResult::KvCacheCopyFailed(description()),
            ),
            (
                DecisionError::KvCacheRemovalFailed(
                    ContextMemoryError::SeqRmPartialSequenceNotRemoved {
                        seq_id: 0,
                        p0: -1,
                        p1: -1,
                    },
                ),
                DecisionResult::KvCacheRemovalFailed(description()),
            ),
            (
                DecisionError::MediaEvaluationFailed(MtmdError::EvalNotEnoughMemory),
                DecisionResult::MediaEvaluationFailed(description()),
            ),
            (
                DecisionError::ModelNotLoaded,
                DecisionResult::ModelNotLoaded(description()),
            ),
            (
                DecisionError::PromptExceedsBatch {
                    batch_tokens: 16,
                    prompt_tokens: 32,
                },
                DecisionResult::RequestRejected(description()),
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
            reported(DecisionError::ClientDisconnected(Box::new(SendError(
                DecisionResult::SchedulerUnavailable(String::new())
            )))),
            None
        );
    }
}
