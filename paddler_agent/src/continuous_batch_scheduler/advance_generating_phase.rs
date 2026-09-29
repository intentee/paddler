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
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;
use crate::continuous_batch_scheduler::completion_check_phase;
use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;
use crate::continuous_batch_scheduler::emit_token_phase;
use crate::continuous_batch_scheduler::sample_outcome::SampleOutcome;
use crate::continuous_batch_scheduler::sample_token_phase::SampleTokenPhase;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;

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
        if !matches!(request.state.phase, ContinuousBatchRequestPhase::Generating) {
            return None;
        }

        if request.state.pending_sampled_token.is_some() {
            return None;
        }

        let batch_index = request.state.i_batch?;

        let raw_token = match (SampleTokenPhase {
            context: self.llama_context,
        })
        .run(request, batch_index, self.candidates)
        {
            SampleOutcome::Sampled(token) => token,
            SampleOutcome::AllCandidatesEliminated => {
                error!(
                    "{:?}: sequence {} sampling exhausted candidates",
                    self.scheduler_context.agent_name,
                    request.sequence_id_guard.sequence_id()
                );
                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::SamplerError(
                        "all token candidates were eliminated during sampling".to_owned(),
                    ),
                ));
            }
            SampleOutcome::GrammarRejected(message) => {
                error!(
                    "{:?}: sequence {} grammar rejected sampled token: {message}",
                    self.scheduler_context.agent_name,
                    request.sequence_id_guard.sequence_id()
                );
                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::GrammarRejectedModelOutput(message),
                ));
            }
            SampleOutcome::Failed(message) => {
                error!(
                    "{:?}: sequence {} sampling error: {message}",
                    self.scheduler_context.agent_name,
                    request.sequence_id_guard.sequence_id()
                );
                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::SamplerError(message),
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

        let completion = completion_check_phase::run(
            progress,
            request.state.max_tokens,
            request.state.sampled_tokens,
            request.token_classifier.usage(),
        );

        if matches!(completion, CompletionCheckOutcome::ReachedMaxTokens) {
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
                emit_token_phase::run(&request.generated_tokens_tx, classified),
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
                request.state.store_pending_token(token);
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
