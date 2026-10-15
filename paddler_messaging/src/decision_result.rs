use serde::Deserialize;
use serde::Serialize;

use crate::decision_answer::DecisionAnswer;
use crate::decision_summary::DecisionSummary;
use crate::oversized_decision_details::OversizedDecisionDetails;
use crate::streamable_result::StreamableResult;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum DecisionResult {
    AgentRuntimeFailed(String),
    BatchAssemblyFailed(String),
    DecodeFailed(String),
    Done(DecisionSummary),
    HiddenStateUnavailable(String),
    InferenceModeMismatch(String),
    InputTokenizationFailed(String),
    KvCacheCopyFailed(String),
    KvCacheRemovalFailed(String),
    ModelNotLoaded(String),
    QuestionAnswered(DecisionAnswer),
    RequestExceedsContext(OversizedDecisionDetails),
    SchedulerUnavailable(String),
    StopRequested,
}

impl StreamableResult for DecisionResult {
    fn is_done(&self) -> bool {
        !matches!(self, Self::QuestionAnswered(_))
    }
}

#[cfg(test)]
mod tests {
    use super::DecisionResult;
    use crate::decision_answer::DecisionAnswer;
    use crate::streamable_result::StreamableResult;

    #[test]
    fn answering_a_question_keeps_the_stream_open() {
        assert!(
            !DecisionResult::QuestionAnswered(DecisionAnswer {
                id: "question".to_owned(),
                probabilities: vec![1.0],
            })
            .is_done()
        );
    }

    #[test]
    fn every_other_result_ends_the_stream() {
        assert!(DecisionResult::ModelNotLoaded("agent: no model is loaded".to_owned()).is_done());
    }
}
