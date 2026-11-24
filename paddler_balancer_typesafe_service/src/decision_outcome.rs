use actix_web::http::StatusCode;

use paddler_balancer::compatibility::upstream_failure::UpstreamFailure;
use paddler_messaging::decision_final_result::DecisionFinalResult;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;

use crate::typesafe_decision_failure::TypeSafeDecisionFailure;

const fn failure(status_code: StatusCode, detail: String) -> DecisionOutcome {
    DecisionOutcome::Failed(TypeSafeDecisionFailure {
        detail,
        status_code,
    })
}

#[derive(Debug, Eq, PartialEq)]
pub enum DecisionOutcome {
    Failed(TypeSafeDecisionFailure),
    Finished(DecisionSummary),
}

impl From<DecisionFinalResult> for DecisionOutcome {
    fn from(decision_final_result: DecisionFinalResult) -> Self {
        match decision_final_result {
            DecisionFinalResult::Done(summary) => Self::Finished(summary),
            DecisionFinalResult::RequestExceedsContext(OversizedDecisionDetails {
                context_size,
                required_tokens,
            }) => failure(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!(
                    "the request needs {required_tokens} tokens in the cache, more than the {context_size} the context holds"
                ),
            ),
            DecisionFinalResult::InputTokenizationFailed(detail) => {
                failure(StatusCode::UNPROCESSABLE_ENTITY, detail)
            }
            DecisionFinalResult::DecisionsNotServed(detail)
            | DecisionFinalResult::ModelNotLoaded(detail)
            | DecisionFinalResult::SchedulerUnavailable(detail) => {
                failure(UpstreamFailure::Unavailable.status_code(), detail)
            }
            DecisionFinalResult::BatchAssemblyFailed(detail)
            | DecisionFinalResult::DecodeFailed(detail)
            | DecisionFinalResult::HiddenStateUnavailable(detail)
            | DecisionFinalResult::KvCacheCopyFailed(detail)
            | DecisionFinalResult::KvCacheRemovalFailed(detail) => {
                failure(UpstreamFailure::AgentFailed.status_code(), detail)
            }
            DecisionFinalResult::StopRequested => failure(
                UpstreamFailure::RelayFailed.status_code(),
                "the agent stopped before it finished the decision".to_owned(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;

    use paddler_messaging::decision_final_result::DecisionFinalResult;
    use paddler_messaging::decision_summary::DecisionSummary;
    use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;

    use super::DecisionOutcome;
    use crate::typesafe_decision_failure::TypeSafeDecisionFailure;

    #[test]
    fn classifies_every_final_decision_result() {
        let summary = DecisionSummary {
            input_tokens: 1,
            processing_milliseconds: 1,
        };

        assert_eq!(
            DecisionOutcome::from(DecisionFinalResult::Done(summary.clone())),
            DecisionOutcome::Finished(summary)
        );

        for (decision_final_result, expected_status_code) in [
            (
                DecisionFinalResult::RequestExceedsContext(OversizedDecisionDetails {
                    context_size: 128,
                    required_tokens: 410,
                }),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                DecisionFinalResult::InputTokenizationFailed("nul byte".to_owned()),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                DecisionFinalResult::DecisionsNotServed("decisions not served".to_owned()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                DecisionFinalResult::ModelNotLoaded("no model".to_owned()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                DecisionFinalResult::SchedulerUnavailable("stopped".to_owned()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                DecisionFinalResult::BatchAssemblyFailed("batch".to_owned()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionFinalResult::DecodeFailed("decode".to_owned()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionFinalResult::HiddenStateUnavailable("hidden".to_owned()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionFinalResult::KvCacheCopyFailed("copy".to_owned()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionFinalResult::KvCacheRemovalFailed("removal".to_owned()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (DecisionFinalResult::StopRequested, StatusCode::BAD_GATEWAY),
        ] {
            assert!(matches!(
                DecisionOutcome::from(decision_final_result),
                DecisionOutcome::Failed(TypeSafeDecisionFailure { status_code, .. })
                    if status_code == expected_status_code
            ));
        }
    }
}
