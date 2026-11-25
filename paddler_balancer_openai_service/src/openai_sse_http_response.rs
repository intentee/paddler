use actix_web::HttpResponse;
use actix_web::http::header;
use actix_web_lab::sse;
use futures::stream::Stream;
use serde_json::Error as SerdeJsonError;

pub fn openai_sse_http_response<TStream>(sse_events: TStream) -> HttpResponse
where
    TStream: Stream<Item = Result<sse::Event, SerdeJsonError>> + 'static,
{
    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(sse::Sse::from_stream(sse_events))
}
