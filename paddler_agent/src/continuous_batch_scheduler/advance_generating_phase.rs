use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use log::error;
use log::warn;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
use crate::continuous_batch_scheduler::classify_token_phase;
use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;
use crate::continuous_batch_scheduler::completion_check_phase::CompletionCheckPhase;
use crate::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;
use crate::continuous_batch_scheduler::emit_token_phase;
use crate::continuous_batch_scheduler::sample_outcome::SampleOutcome;
use crate::continuous_batch_scheduler::sample_token_phase::SampleTokenPhase;
use crate::continuous_batch_scheduler::tool_call_pass;
use crate::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;

pub struct AdvanceGeneratingPhase<'context> {
    pub scheduler_context: &'context ContinuousBatchSchedulerContext,
    pub llama_context: &'context LlamaContext<'context>,
}

impl AdvanceGeneratingPhase<'_> {
    pub fn run(self, requests: &mut [ContinuousBatchActiveRequest]) {
        for request in requests {
            let outcome = self.advance_one(request);
            self.apply_outcome(request, outcome);
        }
    }

    fn advance_one(&self, request: &mut ContinuousBatchActiveRequest) -> Option<AdvanceOutcome> {
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
        .run(request, batch_index)
        {
            SampleOutcome::Sampled(token) => token,
            SampleOutcome::AllCandidatesEliminated => {
                error!(
                    "{:?}: sequence {} sampling exhausted candidates",
                    self.scheduler_context.agent_name, request.state.sequence_id
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
                    self.scheduler_context.agent_name, request.state.sequence_id
                );
                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::GrammarRejectedModelOutput(message),
                ));
            }
            SampleOutcome::Failed(message) => {
                error!(
                    "{:?}: sequence {} sampling error: {message}",
                    self.scheduler_context.agent_name, request.state.sequence_id
                );
                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::SamplerError(message),
                ));
            }
        };

        let classified_outcomes = classify_token_phase::run(request, raw_token);

        let completion_phase = CompletionCheckPhase {
            model: &self.scheduler_context.model,
        };

        let raw_as_sampled = SampledToken::Content(raw_token);
        if matches!(
            completion_phase.run(request, &raw_as_sampled),
            CompletionCheckOutcome::ReachedEog
        ) {
            if let Some(pipeline) = request.tool_call_pipeline.as_mut()
                && !pipeline.buffer_is_empty()
                && let Some(event) = pipeline.finalize_to_generated_event()
                && request.generated_tokens_tx.send(event).is_err()
            {
                warn!(
                    "{:?}: sequence {} client disconnected (receiver dropped) during EOG tool-call flush",
                    self.scheduler_context.agent_name, request.state.sequence_id
                );
                return Some(AdvanceOutcome::ChannelDropped);
            }
            return Some(AdvanceOutcome::Completed(GeneratedTokenResult::Done(
                GenerationSummary {
                    usage: *request.token_classifier.usage(),
                },
            )));
        }

        for classified in &classified_outcomes {
            match emit_token_phase::run(request, classified) {
                EmitTokenOutcome::Emitted(_) => {}
                EmitTokenOutcome::ChannelDropped => {
                    warn!(
                        "{:?}: sequence {} client disconnected (receiver dropped)",
                        self.scheduler_context.agent_name, request.state.sequence_id
                    );
                    return Some(AdvanceOutcome::ChannelDropped);
                }
            }

            if let Some(event) =
                tool_call_pass::run(request.tool_call_pipeline.as_mut(), classified)
                && request.generated_tokens_tx.send(event).is_err()
            {
                warn!(
                    "{:?}: sequence {} client disconnected (receiver dropped)",
                    self.scheduler_context.agent_name, request.state.sequence_id
                );
                return Some(AdvanceOutcome::ChannelDropped);
            }
        }

        match completion_phase.run(request, &raw_as_sampled) {
            CompletionCheckOutcome::ReachedEog | CompletionCheckOutcome::ReachedMaxTokens => {
                if let Some(pipeline) = request.tool_call_pipeline.as_mut()
                    && !pipeline.buffer_is_empty()
                    && let Some(event) = pipeline.finalize_to_generated_event()
                    && request.generated_tokens_tx.send(event).is_err()
                {
                    warn!(
                        "{:?}: sequence {} client disconnected (receiver dropped) during tool-call EOG flush",
                        self.scheduler_context.agent_name, request.state.sequence_id
                    );
                    return Some(AdvanceOutcome::ChannelDropped);
                }
                Some(AdvanceOutcome::Completed(GeneratedTokenResult::Done(
                    GenerationSummary {
                        usage: *request.token_classifier.usage(),
                    },
                )))
            }
            CompletionCheckOutcome::Continue => {
                Some(AdvanceOutcome::SampledAndStored(raw_as_sampled))
            }
        }
    }

    fn apply_outcome(
        &self,
        request: &mut ContinuousBatchActiveRequest,
        outcome: Option<AdvanceOutcome>,
    ) {
        match outcome {
            None => {}
            Some(AdvanceOutcome::SampledAndStored(token)) => {
                request.state.store_pending_token(token);
            }
            Some(AdvanceOutcome::Completed(event)) => {
                request.complete_with_outcome(self.scheduler_context.agent_name.as_deref(), event);
            }
            Some(AdvanceOutcome::ChannelDropped) => {
                request.state.mark_completed();
            }
        }
    }
}
