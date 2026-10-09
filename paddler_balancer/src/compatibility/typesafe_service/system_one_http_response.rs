use std::pin::pin;

use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use futures::stream::Stream;
use futures::stream::StreamExt as _;

use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;
use paddler_typesafe_translation::translated_system_one_request::TranslatedSystemOneRequest;

use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;
use crate::compatibility::typesafe_service::typesafe_error_body::TypeSafeErrorBody;
use crate::compatibility::upstream_failure::UpstreamFailure;

fn error_response(status_code: StatusCode, detail: String) -> HttpResponse {
    TypeSafeErrorBody { detail }.into_http_response(status_code)
}

pub async fn system_one_http_response<TStream>(
    translated: &TranslatedSystemOneRequest,
    decision_results: TStream,
) -> HttpResponse
where
    TStream: Stream<Item = AgentResultStreamEvent<DecisionResult>>,
{
    let mut decision_results = pin!(decision_results);
    let mut answers = Vec::new();

    while let Some(decision_result_event) = decision_results.next().await {
        match decision_result_event {
            AgentResultStreamEvent::Result {
                result: DecisionResult::QuestionAnswered(answer),
                ..
            } => {
                answers.push(answer);
            }
            AgentResultStreamEvent::Result {
                result: DecisionResult::Done(summary),
                ..
            } => {
                return match translated.respond(&answers, &summary) {
                    Ok(system_one_response) => HttpResponse::Ok().json(system_one_response),
                    Err(translation_error) => error_response(
                        UpstreamFailure::RelayFailed.status_code(),
                        translation_error.to_string(),
                    ),
                };
            }
            AgentResultStreamEvent::Result {
                result:
                    DecisionResult::RequestExceedsContext(OversizedDecisionDetails {
                        context_size,
                        required_tokens,
                    }),
                ..
            } => {
                return error_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!(
                        "the request needs {required_tokens} tokens in the cache, more than the {context_size} the context holds"
                    ),
                );
            }
            AgentResultStreamEvent::Result {
                result: DecisionResult::InputTokenizationFailed(detail),
                ..
            } => {
                return error_response(StatusCode::UNPROCESSABLE_ENTITY, detail);
            }
            AgentResultStreamEvent::Result {
                result:
                    DecisionResult::InferenceModeMismatch(detail)
                    | DecisionResult::ModelNotLoaded(detail)
                    | DecisionResult::SchedulerUnavailable(detail),
                ..
            } => return error_response(UpstreamFailure::Unavailable.status_code(), detail),
            AgentResultStreamEvent::Result {
                result:
                    DecisionResult::AgentRuntimeFailed(detail)
                    | DecisionResult::BatchAssemblyFailed(detail)
                    | DecisionResult::DecodeFailed(detail)
                    | DecisionResult::HiddenStateUnavailable(detail)
                    | DecisionResult::KvCacheCopyFailed(detail)
                    | DecisionResult::KvCacheRemovalFailed(detail),
                ..
            } => return error_response(UpstreamFailure::AgentFailed.status_code(), detail),
            AgentResultStreamEvent::Result {
                result: DecisionResult::StopRequested,
                ..
            } => break,
            AgentResultStreamEvent::WireError(wire_error) => {
                return error_response(
                    UpstreamFailure::from(&wire_error).status_code(),
                    wire_error.description,
                );
            }
        }
    }

    error_response(
        UpstreamFailure::RelayFailed.status_code(),
        "the agent stopped before it finished the decision".to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;
    use futures::stream::iter;
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::from_value;
    use serde_json::json;

    use paddler_messaging::decision_answer::DecisionAnswer;
    use paddler_messaging::decision_result::DecisionResult;
    use paddler_messaging::decision_summary::DecisionSummary;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;
    use paddler_typesafe_translation::system_one_request::SystemOneRequest;
    use paddler_typesafe_translation::translated_system_one_request::TranslatedSystemOneRequest;

    use super::system_one_http_response;
    use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;

    fn translated_noul_request() -> TranslatedSystemOneRequest {
        from_value::<SystemOneRequest>(json!({
            "model": "kev-latest",
            "state": "The invoice was paid on time.",
            "questions": {"paid": {"type": "noul"}},
        }))
        .unwrap()
        .translate()
        .unwrap()
    }

    fn decision_result(result: DecisionResult) -> AgentResultStreamEvent<DecisionResult> {
        AgentResultStreamEvent::Result {
            request_id: "decision-request".to_owned(),
            result,
        }
    }

    fn done() -> AgentResultStreamEvent<DecisionResult> {
        decision_result(DecisionResult::Done(DecisionSummary {
            input_tokens: 12,
            processing_milliseconds: 3,
        }))
    }

    fn answered(id: &str) -> AgentResultStreamEvent<DecisionResult> {
        decision_result(DecisionResult::QuestionAnswered(DecisionAnswer {
            id: id.to_owned(),
            probabilities: vec![0.2, 0.8],
        }))
    }

    async fn status_and_body(
        events: Vec<AgentResultStreamEvent<DecisionResult>>,
    ) -> (StatusCode, Value) {
        let response = system_one_http_response(&translated_noul_request(), iter(events)).await;
        let status = response.status();

        (
            status,
            from_slice(&to_bytes(response.into_body()).await.unwrap()).unwrap(),
        )
    }

    #[actix_web::test]
    async fn answers_with_the_translated_answers_once_the_decision_is_done() {
        assert_eq!(
            status_and_body(vec![answered("paid"), done()]).await,
            (
                StatusCode::OK,
                json!({
                    "model": "kev-latest",
                    "answers": {"paid": {"type": "noul", "noul": 0.8}},
                    "usage": {"input_tokens": 12, "output_tokens": 0},
                    "latency_ms": 3,
                })
            )
        );
    }

    #[actix_web::test]
    async fn answers_a_bad_gateway_when_the_answers_do_not_match_the_questions() {
        assert_eq!(
            status_and_body(vec![answered("another-question"), done()])
                .await
                .0,
            StatusCode::BAD_GATEWAY
        );
    }

    #[actix_web::test]
    async fn maps_every_decision_failure_to_its_http_status() {
        let detail = || "failure detail".to_owned();
        let failures = [
            (
                DecisionResult::AgentRuntimeFailed(detail()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionResult::BatchAssemblyFailed(detail()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionResult::DecodeFailed(detail()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionResult::HiddenStateUnavailable(detail()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionResult::InferenceModeMismatch(detail()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                DecisionResult::InputTokenizationFailed(detail()),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                DecisionResult::KvCacheCopyFailed(detail()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionResult::KvCacheRemovalFailed(detail()),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (
                DecisionResult::ModelNotLoaded(detail()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                DecisionResult::RequestExceedsContext(OversizedDecisionDetails {
                    context_size: 256,
                    required_tokens: 900,
                }),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                DecisionResult::SchedulerUnavailable(detail()),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ];

        for (failed_decision_result, expected_status) in failures {
            let (status, body) =
                status_and_body(vec![decision_result(failed_decision_result)]).await;

            assert_eq!(status, expected_status);
            assert!(body["detail"].is_string(), "{body}");
        }
    }

    #[actix_web::test]
    async fn answers_a_wire_error_with_its_upstream_status() {
        assert_eq!(
            status_and_body(vec![AgentResultStreamEvent::WireError(JsonRpcError {
                code: 504,
                description: "wire failure".to_owned(),
            })])
            .await,
            (
                StatusCode::GATEWAY_TIMEOUT,
                json!({"detail": "wire failure"})
            )
        );
    }

    #[actix_web::test]
    async fn answers_a_bad_gateway_when_the_agent_stops_the_decision() {
        assert_eq!(
            status_and_body(vec![
                answered("paid"),
                decision_result(DecisionResult::StopRequested)
            ])
            .await,
            (
                StatusCode::BAD_GATEWAY,
                json!({"detail": "the agent stopped before it finished the decision"})
            )
        );
    }

    #[actix_web::test]
    async fn answers_a_bad_gateway_when_the_stream_ends_without_a_result() {
        assert_eq!(
            status_and_body(vec![answered("paid")]).await,
            (
                StatusCode::BAD_GATEWAY,
                json!({"detail": "the agent stopped before it finished the decision"})
            )
        );
    }
}
