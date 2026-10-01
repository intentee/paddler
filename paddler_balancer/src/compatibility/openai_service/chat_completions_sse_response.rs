use std::convert::Infallible;

use actix_web::HttpResponse;
use actix_web::http::header;
use actix_web_lab::sse;
use futures::stream::Stream;
use futures::stream::StreamExt as _;
use futures::stream::once;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

pub fn chat_completions_sse_response<TStream>(transform_results: TStream) -> HttpResponse
where
    TStream: Stream<Item = TransformResult> + 'static,
{
    let event_stream = transform_results
        .filter_map(|transform_result| async move {
            match transform_result {
                TransformResult::Chunk(chunk) | TransformResult::Error(chunk) => {
                    Some(Ok::<sse::Event, Infallible>(sse::Event::Data(
                        sse::Data::new(chunk),
                    )))
                }
                TransformResult::Discard => None,
            }
        })
        .chain(once(async {
            Ok::<sse::Event, Infallible>(sse::Event::Data(sse::Data::new("[DONE]")))
        }));

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(sse::Sse::from_stream(event_stream))
}

#[cfg(test)]
mod tests {
    use actix_web::body;
    use futures::stream::iter;

    use super::chat_completions_sse_response;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

    #[actix_web::test]
    async fn frames_each_chunk_as_an_sse_data_event_and_terminates_with_done() {
        let response = chat_completions_sse_response(iter([
            TransformResult::Chunk("{\"id\":1}".to_owned()),
            TransformResult::Discard,
            TransformResult::Error("{\"error\":{}}".to_owned()),
        ]));

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/event-stream"
        );

        let body_bytes = body::to_bytes(response.into_body()).await.unwrap();

        assert_eq!(
            body_bytes.as_ref(),
            b"data: {\"id\":1}\n\ndata: {\"error\":{}}\n\ndata: [DONE]\n\n"
        );
    }
}
