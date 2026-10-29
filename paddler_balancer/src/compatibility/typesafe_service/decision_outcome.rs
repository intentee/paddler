use actix_web::http::StatusCode;

use paddler_messaging::decision_final_result::DecisionFinalResult;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;

use crate::compatibility::typesafe_service::typesafe_decision_failure::TypeSafeDecisionFailure;
use crate::compatibility::upstream_failure::UpstreamFailure;

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
            DecisionFinalResult::AgentRuntimeFailed(detail)
            | DecisionFinalResult::BatchAssemblyFailed(detail)
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
    use crate::compatibility::typesafe_service::typesafe_decision_failure::TypeSafeDecisionFailure;

    fn failed(status_code: StatusCode, detail: &str) -> DecisionOutcome {
        DecisionOutcome::Failed(TypeSafeDecisionFailure {
            detail: detail.to_owned(),
            status_code,
        })
    }

    #[test]
    fn classifies_every_final_decision_result() {
        let summary = DecisionSummary {
            input_tokens: 1,
            processing_milliseconds: 1,
        };

        for (decision_final_result, expected_outcome) in [
            (
                DecisionFinalResult::Done(summary.clone()),
                DecisionOutcome::Finished(summary),
            ),
            (
                DecisionFinalResult::RequestExceedsContext(OversizedDecisionDetails {
                    context_size: 128,
                    required_tokens: 410,
                }),
                failed(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "the request needs 410 tokens in the cache, more than the 128 the context holds",
                ),
            ),
            (
                DecisionFinalResult::InputTokenizationFailed("nul byte".to_owned()),
                failed(StatusCode::UNPROCESSABLE_ENTITY, "nul byte"),
            ),
            (
                DecisionFinalResult::DecisionsNotServed("no pointer head".to_owned()),
                failed(StatusCode::SERVICE_UNAVAILABLE, "no pointer head"),
            ),
            (
                DecisionFinalResult::ModelNotLoaded("no model".to_owned()),
                failed(StatusCode::SERVICE_UNAVAILABLE, "no model"),
            ),
            (
                DecisionFinalResult::SchedulerUnavailable("stopped".to_owned()),
                failed(StatusCode::SERVICE_UNAVAILABLE, "stopped"),
            ),
            (
                DecisionFinalResult::AgentRuntimeFailed("runtime".to_owned()),
                failed(StatusCode::INTERNAL_SERVER_ERROR, "runtime"),
            ),
            (
                DecisionFinalResult::BatchAssemblyFailed("batch".to_owned()),
                failed(StatusCode::INTERNAL_SERVER_ERROR, "batch"),
            ),
            (
                DecisionFinalResult::DecodeFailed("decode".to_owned()),
                failed(StatusCode::INTERNAL_SERVER_ERROR, "decode"),
            ),
            (
                DecisionFinalResult::HiddenStateUnavailable("hidden".to_owned()),
                failed(StatusCode::INTERNAL_SERVER_ERROR, "hidden"),
            ),
            (
                DecisionFinalResult::KvCacheCopyFailed("copy".to_owned()),
                failed(StatusCode::INTERNAL_SERVER_ERROR, "copy"),
            ),
            (
                DecisionFinalResult::KvCacheRemovalFailed("removal".to_owned()),
                failed(StatusCode::INTERNAL_SERVER_ERROR, "removal"),
            ),
            (
                DecisionFinalResult::StopRequested,
                failed(
                    StatusCode::BAD_GATEWAY,
                    "the agent stopped before it finished the decision",
                ),
            ),
        ] {
            assert_eq!(
                DecisionOutcome::from(decision_final_result),
                expected_outcome
            );
        }
    }
}
