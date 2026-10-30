use actix_web::HttpResponse;
use actix_web_lab::sse;
use futures::future::ready;
use futures::stream::Stream;
use futures::stream::StreamExt as _;
use futures::stream::iter;
use futures::stream::once;
use nanoid::nanoid;
use serde_json::Error as SerdeJsonError;

use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
use paddler_openai_translation::chat_completion_stream::ChatCompletionStream;
use paddler_openai_translation::generation_outcome::GenerationOutcome;
use paddler_openai_translation::generation_progress::GenerationProgress;

use crate::agent_relay_event::AgentRelayEvent;
use crate::compatibility::openai_service::openai_failure::OpenAIFailure;
use crate::compatibility::openai_service::openai_sse_http_response::openai_sse_http_response;

fn openai_failure_data(openai_failure: OpenAIFailure) -> Vec<Result<sse::Data, SerdeJsonError>> {
    vec![sse::Data::new_json(openai_failure.into_error_body())]
}

fn chat_completion_sse_data(
    chat_completion_stream: &mut ChatCompletionStream,
    request_id: &str,
    agent_relay_event: AgentRelayEvent<GenerationProgress, GeneratedTokenFinalResult>,
) -> Vec<Result<sse::Data, SerdeJsonError>> {
    let chat_completion_chunks = match agent_relay_event {
        AgentRelayEvent::Failed(relay_failure) => {
            return openai_failure_data(OpenAIFailure::Relay(relay_failure));
        }
        AgentRelayEvent::Final(final_result) => {
            chat_completion_stream.conclude(request_id, GenerationOutcome::from(final_result))
        }
        AgentRelayEvent::Partial(generation_progress) => {
            chat_completion_stream.advance(request_id, generation_progress)
        }
    };

    match chat_completion_chunks {
        Ok(chat_completion_chunks) => chat_completion_chunks
            .into_iter()
            .map(sse::Data::new_json)
            .collect(),
        Err(generation_failure) => {
            openai_failure_data(OpenAIFailure::Generation(generation_failure))
        }
    }
}

pub fn chat_completions_sse_response<TAgentRelayEvents>(
    chat_completion_stream: ChatCompletionStream,
    agent_relay_events: TAgentRelayEvents,
) -> HttpResponse
where
    TAgentRelayEvents:
        Stream<Item = AgentRelayEvent<GenerationProgress, GeneratedTokenFinalResult>> + 'static,
{
    let request_id: String = nanoid!();

    openai_sse_http_response(
        agent_relay_events
            .scan(
                chat_completion_stream,
                move |chat_completion_stream, agent_relay_event| {
                    ready(Some(chat_completion_sse_data(
                        chat_completion_stream,
                        &request_id,
                        agent_relay_event,
                    )))
                },
            )
            .flat_map(iter)
            .chain(once(ready(Ok(sse::Data::new("[DONE]")))))
            .map(|sse_data| sse_data.map(sse::Event::Data)),
    )
}
