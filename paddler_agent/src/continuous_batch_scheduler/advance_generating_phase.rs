use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use log::error;
use log::warn;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_generating_state::ContinuousBatchGeneratingState;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::continuous_batch_scheduler::classify_token_phase;
use crate::continuous_batch_scheduler::client_stream_status::ClientStreamStatus;
use crate::continuous_batch_scheduler::completion_check_phase::CompletionCheckPhase;
use crate::continuous_batch_scheduler::emit_classified_tokens;
use crate::continuous_batch_scheduler::sample_outcome::SampleOutcome;
use crate::continuous_batch_scheduler::sample_token_phase::SampleTokenPhase;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;

pub struct AdvanceGeneratingPhase<'context> {
    pub scheduler_context: &'context ContinuousBatchSchedulerContext,
    pub llama_context: &'context LlamaContext<'context>,
}

impl AdvanceGeneratingPhase<'_> {
    pub fn run(self, requests: &mut [ContinuousBatchActiveRequest]) {
        for request in requests {
            let outcome = self.advance_one(request);

            Self::apply_outcome(request, outcome);
        }
    }

    fn warn_channel_dropped(&self, request: &ContinuousBatchActiveRequest) {
        warn!(
            "{:?}: sequence {} client disconnected (receiver dropped)",
            self.scheduler_context.agent_name,
            request.sequence_id_guard.sequence_id()
        );
    }

    fn emit_all(
        &self,
        request: &mut ContinuousBatchActiveRequest,
        classified_tokens: &[ClassifiedToken],
    ) -> Option<AdvanceOutcome> {
        match emit_classified_tokens::run(
            request.tool_call_pipeline.as_mut(),
            &request.generated_tokens_tx,
            classified_tokens,
        ) {
            ClientStreamStatus::Open => None,
            ClientStreamStatus::Dropped => {
                self.warn_channel_dropped(request);

                Some(AdvanceOutcome::ChannelDropped)
            }
        }
    }

    fn finish(&self, request: &mut ContinuousBatchActiveRequest) -> AdvanceOutcome {
        let flushed_tokens = classify_token_phase::flush(request);

        if let Some(outcome) = self.emit_all(request, &flushed_tokens) {
            return outcome;
        }

        if let Some(pipeline) = request.tool_call_pipeline.as_mut()
            && !pipeline.buffer_is_empty()
            && let Some(event) = pipeline.finalize_to_generated_event()
            && request.generated_tokens_tx.send(event).is_err()
        {
            self.warn_channel_dropped(request);

            return AdvanceOutcome::ChannelDropped;
        }

        AdvanceOutcome::Completed(GeneratedTokenResult::Done(GenerationSummary {
            usage: *request.token_classifier.usage(),
        }))
    }

    fn advance_one(&self, request: &mut ContinuousBatchActiveRequest) -> Option<AdvanceOutcome> {
        let ContinuousBatchRequestPhase::Generating(
            ContinuousBatchGeneratingState::AwaitingSample { batch_index },
        ) = &request.state.phase
        else {
            return None;
        };
        let batch_index = *batch_index;

        let raw_token = match (SampleTokenPhase {
            context: self.llama_context,
        })
        .run(request, batch_index)
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

        let completion_phase = CompletionCheckPhase {
            model: &self.scheduler_context.model,
        };
        let raw_as_sampled = SampledToken::Content(raw_token);

        if completion_phase.reached_eog(&raw_as_sampled) {
            return Some(self.finish(request));
        }

        let classified_tokens = match classify_token_phase::run(request, raw_token) {
            Ok(classified_tokens) => classified_tokens,
            Err(error) => {
                error!(
                    "{:?}: sequence {} token classification failed: {error:#}",
                    self.scheduler_context.agent_name,
                    request.sequence_id_guard.sequence_id()
                );

                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::DetokenizationFailed(error.to_string()),
                ));
            }
        };

        if let Some(outcome) = self.emit_all(request, &classified_tokens) {
            return Some(outcome);
        }

        if completion_phase.reached_max_tokens(request) {
            return Some(self.finish(request));
        }

        Some(AdvanceOutcome::SampledAndStored(raw_as_sampled))
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
