use actix_web::HttpResponse;
use actix_web_lab::sse;
use futures::future::ready;
use futures::stream::Stream;
use futures::stream::StreamExt as _;
use futures::stream::iter;
use serde_json::Error as SerdeJsonError;
use serde_json::to_string;

use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
use paddler_openai_translation::generation_outcome::GenerationOutcome;
use paddler_openai_translation::generation_progress::GenerationProgress;
use paddler_openai_translation::responses_stream::ResponsesStream;
use paddler_openai_translation::responses_stream_event::ResponsesStreamEvent;

use crate::agent_relay_event::AgentRelayEvent;
use crate::compatibility::openai_service::openai_sse_http_response::openai_sse_http_response;

fn responses_stream_events(
    responses_stream: &mut ResponsesStream,
    agent_relay_event: AgentRelayEvent<GenerationProgress, GeneratedTokenFinalResult>,
) -> Vec<ResponsesStreamEvent> {
    match agent_relay_event {
        AgentRelayEvent::Failed(relay_failure) => {
            responses_stream.fail(relay_failure.description())
        }
        AgentRelayEvent::Final(final_result) => {
            responses_stream.conclude(GenerationOutcome::from(final_result))
        }
        AgentRelayEvent::Partial(generation_progress) => {
            responses_stream.advance(generation_progress)
        }
    }
}

fn event_to_sse_data(event: &ResponsesStreamEvent) -> Result<sse::Data, SerdeJsonError> {
    to_string(event)
        .map(|serialized_event| sse::Data::new(serialized_event).event(event.event_name()))
}

pub fn responses_sse_response<TAgentRelayEvents>(
    responses_stream: ResponsesStream,
    agent_relay_events: TAgentRelayEvents,
) -> HttpResponse
where
    TAgentRelayEvents:
        Stream<Item = AgentRelayEvent<GenerationProgress, GeneratedTokenFinalResult>> + 'static,
{
    openai_sse_http_response(
        agent_relay_events
            .scan(responses_stream, |responses_stream, agent_relay_event| {
                ready(Some(responses_stream_events(
                    responses_stream,
                    agent_relay_event,
                )))
            })
            .flat_map(iter)
            .map(|event| event_to_sse_data(&event).map(sse::Event::Data)),
    )
}
