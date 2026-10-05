use actix_web::HttpResponse;
use actix_web::http::header;
use actix_web_lab::sse;
use futures::stream::Stream;
use futures::stream::StreamExt as _;
use serde_json::Error;
use serde_json::to_string;

use crate::compatibility::openai_service::responses_stream_event::ResponsesStreamEvent;

fn event_to_sse_data(event: &ResponsesStreamEvent) -> Result<sse::Data, Error> {
    to_string(event)
        .map(|serialized_event| sse::Data::new(serialized_event).event(event.event_name()))
}

pub fn responses_sse_response<TStream>(responses_stream_events: TStream) -> HttpResponse
where
    TStream: Stream<Item = ResponsesStreamEvent> + 'static,
{
    let event_stream =
        responses_stream_events.map(|event| event_to_sse_data(&event).map(sse::Event::Data));

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(sse::Sse::from_stream(event_stream))
}
