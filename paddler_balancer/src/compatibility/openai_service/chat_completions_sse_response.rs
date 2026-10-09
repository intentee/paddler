use actix_web::HttpResponse;
use actix_web_lab::sse;
use futures::future::ready;
use futures::stream::Stream;
use futures::stream::StreamExt as _;
use futures::stream::iter;
use futures::stream::once;
use serde_json::Error as SerdeJsonError;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_openai_translation::chat_completion_stream::ChatCompletionStream;
use paddler_openai_translation::generation_event::GenerationEvent;

use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;
use crate::compatibility::openai_service::openai_failure::OpenAIFailure;
use crate::compatibility::openai_service::openai_sse_http_response::openai_sse_http_response;

fn openai_failure_data(openai_failure: OpenAIFailure) -> Vec<Result<sse::Data, SerdeJsonError>> {
    vec![sse::Data::new_json(openai_failure.into_error_body())]
}

fn chat_completion_sse_data(
    chat_completion_stream: &mut ChatCompletionStream,
    generated_token_event: AgentResultStreamEvent<GeneratedTokenResult>,
) -> Vec<Result<sse::Data, SerdeJsonError>> {
    match generated_token_event {
        AgentResultStreamEvent::Result { request_id, result } => {
            match chat_completion_stream.advance(&request_id, GenerationEvent::from(result)) {
                Ok(chat_completion_chunks) => chat_completion_chunks
                    .into_iter()
                    .map(sse::Data::new_json)
                    .collect(),
                Err(generation_failure) => {
                    openai_failure_data(OpenAIFailure::Generation(generation_failure))
                }
            }
        }
        AgentResultStreamEvent::WireError(wire_error) => {
            openai_failure_data(OpenAIFailure::Wire(wire_error))
        }
    }
}

pub fn chat_completions_sse_response<TStream>(
    chat_completion_stream: ChatCompletionStream,
    generated_tokens: TStream,
) -> HttpResponse
where
    TStream: Stream<Item = AgentResultStreamEvent<GeneratedTokenResult>> + 'static,
{
    openai_sse_http_response(
        generated_tokens
            .scan(
                chat_completion_stream,
                |chat_completion_stream, generated_token_event| {
                    ready(Some(chat_completion_sse_data(
                        chat_completion_stream,
                        generated_token_event,
                    )))
                },
            )
            .flat_map(iter)
            .chain(once(ready(Ok(sse::Data::new("[DONE]")))))
            .map(|sse_data| sse_data.map(sse::Event::Data)),
    )
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use futures::stream::iter;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_openai_translation::chat_completion_header::ChatCompletionHeader;
    use paddler_openai_translation::chat_completion_stream::ChatCompletionStream;
    use paddler_openai_translation::chat_completion_stream_params::ChatCompletionStreamParams;

    use super::chat_completions_sse_response;
    use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;

    #[actix_web::test]
    async fn frames_chunks_and_failures_as_sse_data_events_and_terminates_with_done() {
        let response = chat_completions_sse_response(
            ChatCompletionStream::new(ChatCompletionStreamParams {
                header: ChatCompletionHeader {
                    created: 0,
                    model: "test-model".to_owned(),
                },
                include_usage: false,
                system_fingerprint: "test-fingerprint".to_owned(),
            }),
            iter([
                AgentResultStreamEvent::Result {
                    request_id: "test-request".to_owned(),
                    result: GeneratedTokenResult::ContentToken("hi".to_owned()),
                },
                AgentResultStreamEvent::Result {
                    request_id: "test-request".to_owned(),
                    result: GeneratedTokenResult::ImageDecodingFailed(
                        "unsupported format".to_owned(),
                    ),
                },
                AgentResultStreamEvent::WireError(JsonRpcError {
                    code: 503,
                    description: "balancer is shutting down".to_owned(),
                }),
            ]),
        );

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/event-stream"
        );
        assert_eq!(
            to_bytes(response.into_body()).await.unwrap().as_ref(),
            concat!(
                r#"data: {"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","content":"hi"},"logprobs":null,"finish_reason":null}]}"#,
                "\n\n",
                r#"data: {"error":{"message":"unsupported format","type":"invalid_request_error","param":null,"code":null}}"#,
                "\n\n",
                r#"data: {"error":{"message":"balancer is shutting down","type":"server_error","param":null,"code":null}}"#,
                "\n\n",
                "data: [DONE]\n\n",
            )
            .as_bytes()
        );
    }
}
