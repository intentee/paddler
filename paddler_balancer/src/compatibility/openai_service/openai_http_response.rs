use actix_web::HttpResponse;
use nanoid::nanoid;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_openai_translation::completes_generation::CompletesGeneration;
use paddler_openai_translation::generated_output::GeneratedOutput;
use paddler_openai_translation::generation_outcome::GenerationOutcome;
use paddler_openai_translation::generation_progress::GenerationProgress;

use crate::agent_relay::AgentRelay;
use crate::agent_relay_conclusion::AgentRelayConclusion;
use crate::agent_relay_step::AgentRelayStep;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::compatibility::openai_service::openai_failure::OpenAIFailure;

pub async fn openai_http_response<TCompletesGeneration, TParams>(
    completes_generation: &TCompletesGeneration,
    mut agent_relay: AgentRelay<TParams>,
) -> HttpResponse
where
    TCompletesGeneration: CompletesGeneration,
    TParams: AgentStreamingRequest<Response = GeneratedTokenResult>,
{
    let mut generated_output = GeneratedOutput::default();

    loop {
        match agent_relay.next().await {
            AgentRelayStep::Concluded(AgentRelayConclusion::AgentConcluded {
                final_result,
                ..
            }) => {
                return match GenerationOutcome::from(final_result) {
                    GenerationOutcome::Failed(generation_failure) => {
                        OpenAIFailure::Generation(generation_failure).into_http_response()
                    }
                    GenerationOutcome::Finished(generation_summary) => {
                        let request_id: String = nanoid!();

                        HttpResponse::Ok().json(completes_generation.complete(
                            &request_id,
                            generated_output,
                            &generation_summary,
                        ))
                    }
                };
            }
            AgentRelayStep::Concluded(AgentRelayConclusion::Failed(relay_failure)) => {
                return OpenAIFailure::Relay(relay_failure).into_http_response();
            }
            AgentRelayStep::Partial {
                partial_result,
                relay,
                ..
            } => {
                match GenerationProgress::from(partial_result) {
                    GenerationProgress::Failed(generation_failure) => {
                        return OpenAIFailure::Generation(generation_failure).into_http_response();
                    }
                    GenerationProgress::Produced(generated_output_part) => {
                        generated_output.append(generated_output_part);
                    }
                    GenerationProgress::ToolCallTokenProduced => {}
                }

                agent_relay = relay;
            }
        }
    }
}
