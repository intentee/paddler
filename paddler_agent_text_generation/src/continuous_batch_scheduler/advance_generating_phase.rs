use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::ingest_outcome::IngestOutcome;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;
use log::error;
use log::warn;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;
use crate::continuous_batch_scheduler::completion_check_phase::CompletionCheckPhase;
use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;
use crate::continuous_batch_scheduler::emit_token_phase::EmitTokenPhase;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use crate::sampling_outcome::SamplingOutcome;

pub struct AdvanceGeneratingPhase<'context> {
    pub candidates: &'context mut LlamaTokenDataArray,
    pub ingest_outcomes: &'context mut Vec<IngestOutcome>,
    pub llama_context: &'context LlamaContext<'context>,
    pub scheduler_context: &'context ContinuousBatchSchedulerContext,
}

impl AdvanceGeneratingPhase<'_> {
    pub fn run(mut self, requests: &mut [ContinuousBatchActiveRequest]) {
        for request in requests {
            let outcome = self.advance_one(request);

            Self::apply_outcome(request, outcome);
        }
    }

    fn advance_one(
        &mut self,
        request: &mut ContinuousBatchActiveRequest,
    ) -> Option<AdvanceOutcome> {
        let ContinuousBatchRequestPhase::Generating(ContinuousBatchGenerationStep::ReadyToSample {
            batch_position,
        }) = request.state.phase
        else {
            return None;
        };

        let raw_token =
            match request
                .token_sampling
                .sample(self.llama_context, batch_position, self.candidates)
            {
                Ok(SamplingOutcome::Token(token)) => token,
                Ok(SamplingOutcome::AllCandidatesEliminated) => {
                    error!(
                        "{:?}: sequence {} sampling exhausted candidates",
                        self.scheduler_context.agent_name,
                        request.sequence_id_guard.sequence_id()
                    );
                    return Some(AdvanceOutcome::Completed(
                        GeneratedTokenResult::SamplingCandidatesExhausted(
                            "all token candidates were eliminated during sampling".to_owned(),
                        ),
                    ));
                }
                Ok(SamplingOutcome::GrammarRejectedModelOutput(message)) => {
                    error!(
                        "{:?}: sequence {} grammar rejected sampled token: {message}",
                        self.scheduler_context.agent_name,
                        request.sequence_id_guard.sequence_id()
                    );
                    return Some(AdvanceOutcome::Completed(
                        GeneratedTokenResult::GrammarRejectedModelOutput(message),
                    ));
                }
                Err(sampling_error) => {
                    return Some(AdvanceOutcome::Completed(
                        sampling_error.into_generated_token_result(
                            self.scheduler_context.agent_name.as_deref(),
                        ),
                    ));
                }
            };

        request.state.record_sampled_token();
        self.ingest_outcomes.clear();

        let progress = match request
            .token_classifier
            .ingest(raw_token, self.ingest_outcomes)
        {
            Ok(progress) => progress,
            Err(detokenization_error) => {
                error!(
                    "{:?}: sequence {} token classification failed: {detokenization_error}",
                    self.scheduler_context.agent_name,
                    request.sequence_id_guard.sequence_id()
                );

                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::DetokenizationFailed(detokenization_error.to_string()),
                ));
            }
        };

        let completion = CompletionCheckPhase {
            request_state: &request.state,
            sequence_context_size: self.llama_context.n_ctx_seq(),
            usage: request.token_classifier.usage(),
        }
        .run(progress);

        if matches!(
            completion,
            CompletionCheckOutcome::ReachedContextLimit | CompletionCheckOutcome::ReachedMaxTokens
        ) {
            request.token_classifier.finish(self.ingest_outcomes);
        }

        if self.client_disconnected_while_emitting(request) {
            return Some(AdvanceOutcome::ChannelDropped);
        }

        match completion {
            CompletionCheckOutcome::Continue => Some(AdvanceOutcome::SampledAndStored(
                SampledToken::Content(raw_token),
            )),
            CompletionCheckOutcome::ReachedEndOfGeneration => {
                Some(self.complete_after_resolving_tool_calls(
                    request,
                    GenerationFinish::EndOfGeneration,
                ))
            }
            CompletionCheckOutcome::ReachedContextLimit => Some(
                self.complete_after_resolving_tool_calls(request, GenerationFinish::ContextFull),
            ),
            CompletionCheckOutcome::ReachedMaxTokens => {
                Some(self.complete_after_resolving_tool_calls(request, GenerationFinish::MaxTokens))
            }
        }
    }

    fn client_disconnected_while_emitting(
        &mut self,
        request: &mut ContinuousBatchActiveRequest,
    ) -> bool {
        for ingest_outcome in self.ingest_outcomes.drain(..) {
            let classified =
                ClassifiedToken::classify(ingest_outcome, &mut request.state.last_outcome_section);

            request.tool_call_handling.feed(&classified);

            let tool_call_result = request
                .tool_call_handling
                .resolve_on_section_exit(&classified);

            if matches!(
                EmitTokenPhase {
                    generated_tokens_tx: &request.generated_tokens_tx,
                }
                .run(classified),
                EmitTokenOutcome::ChannelDropped
            ) || tool_call_result.is_some_and(|tool_call_result| {
                request.generated_tokens_tx.send(tool_call_result).is_err()
            }) {
                warn!(
                    "{:?}: sequence {} client disconnected (receiver dropped)",
                    self.scheduler_context.agent_name,
                    request.sequence_id_guard.sequence_id()
                );

                return true;
            }
        }

        false
    }

    fn complete_after_resolving_tool_calls(
        &self,
        request: &mut ContinuousBatchActiveRequest,
        finish: GenerationFinish,
    ) -> AdvanceOutcome {
        if let Some(tool_call_result) = request.tool_call_handling.resolve_at_completion()
            && request.generated_tokens_tx.send(tool_call_result).is_err()
        {
            warn!(
                "{:?}: sequence {} client disconnected (receiver dropped) while resolving tool calls at completion",
                self.scheduler_context.agent_name,
                request.sequence_id_guard.sequence_id()
            );

            return AdvanceOutcome::ChannelDropped;
        }

        AdvanceOutcome::Completed(GeneratedTokenResult::Done(GenerationSummary {
            finish,
            usage: *request.token_classifier.usage(),
        }))
    }

    fn apply_outcome(request: &mut ContinuousBatchActiveRequest, outcome: Option<AdvanceOutcome>) {
        match outcome {
            None => {}
            Some(AdvanceOutcome::SampledAndStored(token)) => {
                request.state.await_decode_of(token);
            }
            Some(AdvanceOutcome::Completed(event)) => {
                request.complete_with_outcome(event);
            }
            Some(AdvanceOutcome::ChannelDropped) => {
                request
                    .state
                    .mark_completed(ContinuousBatchTerminalOutcome::EmitNothing);
            }
        }
    }
}
