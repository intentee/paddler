use std::pin::pin;

use actix_web::HttpResponse;
use futures::stream::Stream;
use futures::stream::StreamExt as _;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_openai_translation::completes_generation::CompletesGeneration;
use paddler_openai_translation::generated_output::GeneratedOutput;
use paddler_openai_translation::generation_event::GenerationEvent;

use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;
use crate::compatibility::openai_service::openai_failure::OpenAIFailure;

pub async fn openai_http_response<TCompletesGeneration, TStream>(
    completes_generation: &TCompletesGeneration,
    generated_tokens: TStream,
) -> HttpResponse
where
    TCompletesGeneration: CompletesGeneration,
    TStream: Stream<Item = AgentResultStreamEvent<GeneratedTokenResult>>,
{
    let mut generated_tokens = pin!(generated_tokens);
    let mut generated_output = GeneratedOutput::default();

    while let Some(generated_token_event) = generated_tokens.next().await {
        match generated_token_event {
            AgentResultStreamEvent::Result { request_id, result } => {
                match GenerationEvent::from(result) {
                    GenerationEvent::Produced(generated_output_part) => {
                        generated_output.append(generated_output_part);
                    }
                    GenerationEvent::ToolCallTokenProduced => {}
                    GenerationEvent::Finished(generation_summary) => {
                        return HttpResponse::Ok().json(completes_generation.complete(
                            &request_id,
                            generated_output,
                            &generation_summary,
                        ));
                    }
                    GenerationEvent::Failed(generation_failure) => {
                        return OpenAIFailure::Generation(generation_failure).into_http_response();
                    }
                }
            }
            AgentResultStreamEvent::WireError(wire_error) => {
                return OpenAIFailure::Wire(wire_error).into_http_response();
            }
        }
    }

    OpenAIFailure::Unfinished.into_http_response()
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;
    use futures::stream::iter;
    use llama_cpp_bindings_types::TokenUsage;
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::json;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_openai_translation::chat_completion_header::ChatCompletionHeader;

    use super::openai_http_response;
    use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;

    fn generated(result: GeneratedTokenResult) -> AgentResultStreamEvent<GeneratedTokenResult> {
        AgentResultStreamEvent::Result {
            request_id: "test-request".to_owned(),
            result,
        }
    }

    async fn status_and_body(
        events: Vec<AgentResultStreamEvent<GeneratedTokenResult>>,
    ) -> (StatusCode, Value) {
        let response = openai_http_response(
            &ChatCompletionHeader {
                created: 0,
                model: "test-model".to_owned(),
            },
            iter(events),
        )
        .await;
        let status = response.status();

        (
            status,
            from_slice(&to_bytes(response.into_body()).await.unwrap()).unwrap(),
        )
    }

    #[actix_web::test]
    async fn answers_with_the_completion_once_the_generation_finishes() {
        let (status, body) = status_and_body(vec![
            generated(GeneratedTokenResult::ContentToken("hel".to_owned())),
            generated(GeneratedTokenResult::ToolCallToken("{".to_owned())),
            generated(GeneratedTokenResult::ContentToken("lo".to_owned())),
            generated(GeneratedTokenResult::Done(GenerationSummary {
                finish: GenerationFinish::EndOfGeneration,
                usage: TokenUsage::new(),
            })),
        ])
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], "test-request");
        assert_eq!(body["choices"][0]["message"]["content"], "hello");
    }

    #[actix_web::test]
    async fn answers_a_generation_failure_with_its_status() {
        assert_eq!(
            status_and_body(vec![generated(GeneratedTokenResult::ModelNotLoaded(
                "no model is loaded".to_owned()
            ))])
            .await,
            (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({"error": {"message": "no model is loaded", "type": "server_error", "param": null, "code": null}})
            )
        );
    }

    #[actix_web::test]
    async fn answers_a_wire_error_with_its_upstream_status() {
        assert_eq!(
            status_and_body(vec![AgentResultStreamEvent::WireError(JsonRpcError {
                code: 503,
                description: "Buffered requests overflow".to_owned(),
            })])
            .await
            .0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[actix_web::test]
    async fn answers_a_bad_gateway_when_the_stream_ends_without_a_result() {
        assert_eq!(
            status_and_body(vec![generated(GeneratedTokenResult::ContentToken(
                "hel".to_owned()
            ))])
            .await
            .0,
            StatusCode::BAD_GATEWAY
        );
    }
}
