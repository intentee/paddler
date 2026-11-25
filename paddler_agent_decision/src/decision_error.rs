use std::ffi::c_int;
use std::num::ParseFloatError;
use std::num::TryFromIntError;

use llama_cpp_context::context_error::ContextError;
use llama_cpp_context_memory::context_memory_error::ContextMemoryError;
use llama_cpp_model::model_error::ModelError;
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

    #[error("failed to add a decision token to the batch")]
    BatchAssemblyFailed(#[source] ContextError),

    #[error("the client stopped listening for decisions")]
    ClientDisconnected(#[source] SendError<DecisionResult>),

    #[error("the agent serves no decisions with the loaded model")]
    DecisionsNotServed,

    #[error("llama.cpp rejected the decision batch")]
    DecodeFailed(#[source] ContextError),

    #[error("unable to read the attributes of the decision delimiter {delimiter}")]
    DelimiterAttributesUnreadable {
        delimiter: String,
        #[source]
        source: ModelError,
    },

    #[error("the decision delimiter {delimiter} is not a single control token of the model")]
    DelimiterIsNotAControlToken { delimiter: String },

    #[error("unable to tokenize the decision delimiter {delimiter}")]
    DelimiterTokenizationFailed {
        delimiter: String,
        #[source]
        source: ModelError,
    },

    #[error("unable to read the hidden state of a decision output")]
    HiddenStateUnavailable(#[source] ContextError),

    #[error("failed to tokenize the decision input")]
    InputTokenizationFailed(#[source] ModelError),

    #[error("the metadata of the kev decision model names no architecture")]
    KevArchitectureUnknown,

    #[error(
        "half of the {n_embd_out} outputs of the kev decision model does not fit a head dimension"
    )]
    KevHeadDimensionUnrepresentable {
        n_embd_out: c_int,
        #[source]
        source: TryFromIntError,
    },

    #[error("the kev decision model holds no {key} temperature")]
    KevTemperatureMissing { key: String },

    #[error(
        "the kev decision model declares different temperatures for choices ({choice}), scores ({score}) and noul questions ({noul}), and decisions carry no question type to pick one"
    )]
    KevTemperaturesDiffer { choice: f32, noul: f32, score: f32 },

    #[error(
        "the kev decision model declares {key} as {temperature}, which is not a finite positive temperature"
    )]
    KevTemperatureOutOfRange { key: String, temperature: f32 },

    #[error("the kev decision model declares {key} as {declared:?}, which is not a number")]
    KevTemperatureUnparseable {
        declared: String,
        key: String,
        #[source]
        source: ParseFloatError,
    },

    #[error("failed to copy the decision state into its question lane")]
    KvCacheCopyFailed(#[source] ContextMemoryError),

    #[error("failed to remove a decision sequence from the KV cache")]
    KvCacheRemovalFailed(#[source] ContextMemoryError),

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error(
        "the decision needs {} tokens in the cache, more than the {} the context holds",
        details.required_tokens,
        details.context_size
    )]
    RequestExceedsContext { details: OversizedDecisionDetails },

    #[error("the scheduler is no longer accepting requests")]
    SchedulerUnavailable,
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
            Self::AgentRuntime(_)
            | Self::DelimiterAttributesUnreadable { .. }
            | Self::DelimiterIsNotAControlToken { .. }
            | Self::DelimiterTokenizationFailed { .. }
            | Self::KevArchitectureUnknown
            | Self::KevHeadDimensionUnrepresentable { .. }
            | Self::KevTemperatureMissing { .. }
            | Self::KevTemperatureOutOfRange { .. }
            | Self::KevTemperaturesDiffer { .. }
            | Self::KevTemperatureUnparseable { .. }
            | Self::ModelNotLoaded => DecisionResult::ModelNotLoaded(message),
            Self::BatchAssemblyFailed(_) => DecisionResult::BatchAssemblyFailed(message),
            Self::ClientDisconnected(_) => return,
            Self::DecisionsNotServed => DecisionResult::DecisionsNotServed(message),
            Self::DecodeFailed(_) => DecisionResult::DecodeFailed(message),
            Self::HiddenStateUnavailable(_) => DecisionResult::HiddenStateUnavailable(message),
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
    use std::mem::discriminant;

    use llama_cpp_context::context_error::ContextError;
    use llama_cpp_context_memory::context_memory_error::ContextMemoryError;
    use llama_cpp_model::model_error::ModelError;
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
                DecisionError::HiddenStateUnavailable(ContextError::EmbeddingTokenNotAnOutput {
                    token_index: 0,
                }),
                DecisionResult::HiddenStateUnavailable(description()),
            ),
            (
                DecisionError::InputTokenizationFailed(ModelError::TokenizeNotEnoughMemory),
                DecisionResult::InputTokenizationFailed(description()),
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
