use actix_web::HttpResponse;
use actix_web_lab::sse;
use futures::future::ready;
use futures::stream::Stream;
use futures::stream::StreamExt as _;
use futures::stream::iter;
use serde_json::Error as SerdeJsonError;
use serde_json::to_string;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_openai_translation::generation_event::GenerationEvent;
use paddler_openai_translation::responses_stream::ResponsesStream;
use paddler_openai_translation::responses_stream_event::ResponsesStreamEvent;

use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;
use crate::compatibility::openai_service::openai_sse_http_response::openai_sse_http_response;

fn responses_stream_events(
    responses_stream: &mut ResponsesStream,
    generated_token_event: AgentResultStreamEvent<GeneratedTokenResult>,
) -> Vec<ResponsesStreamEvent> {
    match generated_token_event {
        AgentResultStreamEvent::Result { result, .. } => {
            responses_stream.advance(GenerationEvent::from(result))
        }
        AgentResultStreamEvent::WireError(wire_error) => {
            responses_stream.fail(wire_error.description)
        }
    }
}

fn event_to_sse_data(event: &ResponsesStreamEvent) -> Result<sse::Data, SerdeJsonError> {
    to_string(event)
        .map(|serialized_event| sse::Data::new(serialized_event).event(event.event_name()))
}

pub fn responses_sse_response<TStream>(
    responses_stream: ResponsesStream,
    generated_tokens: TStream,
) -> HttpResponse
where
    TStream: Stream<Item = AgentResultStreamEvent<GeneratedTokenResult>> + 'static,
{
    openai_sse_http_response(
        generated_tokens
            .scan(
                responses_stream,
                |responses_stream, generated_token_event| {
                    ready(Some(responses_stream_events(
                        responses_stream,
                        generated_token_event,
                    )))
                },
            )
            .flat_map(iter)
            .map(|event| event_to_sse_data(&event).map(sse::Event::Data)),
    )
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use futures::stream::iter;
    use serde_json::Value;
    use serde_json::from_str;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_openai_translation::responses_response_header::ResponsesResponseHeader;
    use paddler_openai_translation::responses_stream::ResponsesStream;

    use super::responses_sse_response;
    use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;

    #[actix_web::test]
    async fn names_each_event_and_reports_wire_errors_as_a_failed_response() {
        let response = responses_sse_response(
            ResponsesStream::new(ResponsesResponseHeader {
                id: "resp_test".to_owned(),
                created_at: 0,
                model: "test-model".to_owned(),
                instructions: None,
                temperature: 0.25,
                top_p: 0.5,
            }),
            iter([
                AgentResultStreamEvent::Result {
                    request_id: "test-request".to_owned(),
                    result: GeneratedTokenResult::ContentToken("hi".to_owned()),
                },
                AgentResultStreamEvent::WireError(JsonRpcError {
                    code: 504,
                    description: "timed out".to_owned(),
                }),
            ]),
        );

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/event-stream"
        );

        let body =
            String::from_utf8(to_bytes(response.into_body()).await.unwrap().to_vec()).unwrap();
        let events: Vec<(&str, Value)> = body
            .split_terminator("\n\n")
            .map(|sse_event| {
                let (event_line, data_line) = sse_event.split_once('\n').unwrap();

                (
                    event_line.strip_prefix("event: ").unwrap(),
                    from_str(data_line.strip_prefix("data: ").unwrap()).unwrap(),
                )
            })
            .collect();

        assert_eq!(
            events
                .iter()
                .map(|(event_name, _)| *event_name)
                .collect::<Vec<_>>(),
            vec![
                "response.created",
                "response.in_progress",
                "response.output_item.added",
                "response.content_part.added",
                "response.output_text.delta",
                "response.failed",
            ]
        );
        assert_eq!(events[5].1["response"]["error"]["message"], "timed out");
    }
}
