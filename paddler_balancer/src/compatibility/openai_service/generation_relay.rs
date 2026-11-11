use actix_web::HttpResponse;
use futures::stream::Stream;
use futures::stream::unfold;
use nanoid::nanoid;

use paddler_messaging::generated_token_final_result::GeneratedTokenFinalResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_openai_translation::completes_generation::CompletesGeneration;
use paddler_openai_translation::generated_output::GeneratedOutput;
use paddler_openai_translation::generation_outcome::GenerationOutcome;
use paddler_openai_translation::generation_progress::GenerationProgress;

use crate::agent_relay::AgentRelay;
use crate::agent_relay_conclusion::AgentRelayConclusion;
use crate::agent_relay_event::AgentRelayEvent;
use crate::agent_relay_step::AgentRelayStep;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::compatibility::openai_service::generation_progress_phase::GenerationProgressPhase;
use crate::compatibility::openai_service::openai_failure::OpenAIFailure;

pub struct GenerationRelay<TParams>
where
    TParams: AgentStreamingRequest<Response = GeneratedTokenResult>,
{
    pub agent_relay: AgentRelay<TParams>,
}

impl<TParams> GenerationRelay<TParams>
where
    TParams: AgentStreamingRequest<Response = GeneratedTokenResult>,
{
    pub async fn into_http_response<TCompletesGeneration>(
        self,
        completes_generation: &TCompletesGeneration,
    ) -> HttpResponse
    where
        TCompletesGeneration: CompletesGeneration + Sync,
    {
        let mut agent_relay = self.agent_relay;
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
                            return OpenAIFailure::Generation(generation_failure)
                                .into_http_response();
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

    pub fn into_progress_events(
        self,
    ) -> impl Stream<Item = AgentRelayEvent<GenerationProgress, GeneratedTokenFinalResult>> {
        let Self { agent_relay } = self;
        let settlement = agent_relay.settlement();

        unfold(
            GenerationProgressPhase::Relaying {
                agent_relay,
                settlement,
            },
            |generation_progress_phase| async move {
                match generation_progress_phase {
                    GenerationProgressPhase::Relaying {
                        agent_relay,
                        settlement,
                    } => match agent_relay.next().await {
                        AgentRelayStep::Concluded(AgentRelayConclusion::AgentConcluded {
                            final_result,
                            ..
                        }) => Some((
                            AgentRelayEvent::Final(final_result),
                            GenerationProgressPhase::Settling { settlement },
                        )),
                        AgentRelayStep::Concluded(AgentRelayConclusion::Failed(relay_failure)) => {
                            Some((
                                AgentRelayEvent::Failed(relay_failure),
                                GenerationProgressPhase::Settling { settlement },
                            ))
                        }
                        AgentRelayStep::Partial {
                            partial_result,
                            relay,
                            ..
                        } => match GenerationProgress::from(partial_result) {
                            failed_generation @ GenerationProgress::Failed(_) => Some((
                                AgentRelayEvent::Partial(failed_generation),
                                GenerationProgressPhase::ReleasingRelay { relay, settlement },
                            )),
                            generation_progress => Some((
                                AgentRelayEvent::Partial(generation_progress),
                                GenerationProgressPhase::Relaying {
                                    agent_relay: relay,
                                    settlement,
                                },
                            )),
                        },
                    },
                    GenerationProgressPhase::ReleasingRelay { relay, settlement } => {
                        drop(relay);
                        settlement.cancelled().await;

                        None
                    }
                    GenerationProgressPhase::Settling { settlement } => {
                        settlement.cancelled().await;

                        None
                    }
                }
            },
        )
    }
}
