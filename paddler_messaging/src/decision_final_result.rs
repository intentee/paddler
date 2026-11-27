use crate::decision_summary::DecisionSummary;
use crate::oversized_decision_details::OversizedDecisionDetails;

#[derive(Debug, Eq, PartialEq)]
pub enum DecisionFinalResult {
    AnswersUnreadable(String),
    BatchAssemblyFailed(String),
    DecisionsNotServed(String),
    DecodeFailed(String),
    Done(DecisionSummary),
    KvCacheCopyFailed(String),
    KvCacheRemovalFailed(String),
    MediaEvaluationFailed(String),
    ModelNotLoaded(String),
    RequestExceedsContext(OversizedDecisionDetails),
    RequestRejected(String),
    SchedulerUnavailable(String),
    StopRequested,
}
