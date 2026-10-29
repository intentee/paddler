use crate::decision_summary::DecisionSummary;
use crate::oversized_decision_details::OversizedDecisionDetails;

#[derive(Debug, Eq, PartialEq)]
pub enum DecisionFinalResult {
    AgentRuntimeFailed(String),
    BatchAssemblyFailed(String),
    DecisionsNotServed(String),
    DecodeFailed(String),
    Done(DecisionSummary),
    HiddenStateUnavailable(String),
    InputTokenizationFailed(String),
    KvCacheCopyFailed(String),
    KvCacheRemovalFailed(String),
    ModelNotLoaded(String),
    RequestExceedsContext(OversizedDecisionDetails),
    SchedulerUnavailable(String),
    StopRequested,
}
