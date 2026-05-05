use llama_cpp_bindings::context::LlamaContext;
use log::error;
use log::warn;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::generation_summary::GenerationSummary;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::agent::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
use crate::agent::continuous_batch_scheduler::classify_token_phase::ClassifyTokenPhase;
use crate::agent::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;
use crate::agent::continuous_batch_scheduler::completion_check_phase::CompletionCheckPhase;
use crate::agent::continuous_batch_scheduler::emit_token_outcome::EmitTokenOutcome;
use crate::agent::continuous_batch_scheduler::emit_token_phase::EmitTokenPhase;
use crate::agent::continuous_batch_scheduler::sample_outcome::SampleOutcome;
use crate::agent::continuous_batch_scheduler::sample_token_phase::SampleTokenPhase;
use crate::agent::continuous_batch_scheduler::tool_call_pass::ToolCallPass;
use crate::agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;

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
        if !matches!(request.phase, ContinuousBatchRequestPhase::Generating) {
            return None;
        }

        if request.pending_sampled_token.is_some() {
            return None;
        }

        let batch_index = request.i_batch?;

        let raw_token = match (SampleTokenPhase {
            context: self.llama_context,
        })
        .run(request, batch_index)
        {
            SampleOutcome::Sampled(token) => token,
            SampleOutcome::AllCandidatesEliminated => {
                error!(
                    "{:?}: sequence {} sampling exhausted candidates",
                    self.scheduler_context.agent_name, request.sequence_id
                );
                return Some(AdvanceOutcome::Completed(GeneratedTokenResult::SamplerError(
                    "all token candidates were eliminated during sampling".to_owned(),
                )));
            }
            SampleOutcome::GrammarRejected(message) => {
                error!(
                    "{:?}: sequence {} grammar rejected sampled token: {message}",
                    self.scheduler_context.agent_name, request.sequence_id
                );
                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::GrammarRejectedModelOutput(message),
                ));
            }
            SampleOutcome::Failed(message) => {
                error!(
                    "{:?}: sequence {} sampling error: {message}",
                    self.scheduler_context.agent_name, request.sequence_id
                );
                return Some(AdvanceOutcome::Completed(GeneratedTokenResult::SamplerError(
                    message,
                )));
            }
        };

        let classified = ClassifyTokenPhase.run(request, raw_token);

        let completion_phase = CompletionCheckPhase {
            model: &self.scheduler_context.model,
        };

        if matches!(
            completion_phase.run(request, &classified.sampled_token),
            CompletionCheckOutcome::ReachedEog
        ) {
            return Some(AdvanceOutcome::Completed(GeneratedTokenResult::Done(
                GenerationSummary {
                    usage: *request.token_classifier.usage(),
                },
            )));
        }

        let piece = match (EmitTokenPhase {
            model: &self.scheduler_context.model,
        })
        .run(request, &classified)
        {
            EmitTokenOutcome::Emitted(piece) => piece,
            EmitTokenOutcome::PieceConversionFailed(message) => {
                error!(
                    "{:?}: sequence {} token_to_piece failed: {message}",
                    self.scheduler_context.agent_name, request.sequence_id
                );
                return Some(AdvanceOutcome::Completed(GeneratedTokenResult::SamplerError(
                    format!("Failed to convert token to string: {message}"),
                )));
            }
            EmitTokenOutcome::ChannelDropped => {
                warn!(
                    "{:?}: sequence {} client disconnected (receiver dropped)",
                    self.scheduler_context.agent_name, request.sequence_id
                );
                return Some(AdvanceOutcome::ChannelDropped);
            }
        };

        if let Some(event) = ToolCallPass.run(request.tool_call_pipeline.as_mut(), &classified, &piece)
            && request.generated_tokens_tx.send(event).is_err()
        {
            warn!(
                "{:?}: sequence {} client disconnected (receiver dropped)",
                self.scheduler_context.agent_name, request.sequence_id
            );
            return Some(AdvanceOutcome::ChannelDropped);
        }

        match completion_phase.run(request, &classified.sampled_token) {
            CompletionCheckOutcome::ReachedEog | CompletionCheckOutcome::ReachedMaxTokens => {
                Some(AdvanceOutcome::Completed(GeneratedTokenResult::Done(
                    GenerationSummary {
                        usage: *request.token_classifier.usage(),
                    },
                )))
            }
            CompletionCheckOutcome::Continue => {
                Some(AdvanceOutcome::SampledAndStored(classified.sampled_token))
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
                request.pending_sampled_token = Some(token);
            }
            Some(AdvanceOutcome::Completed(event)) => {
                request.complete_with_outcome(&self.scheduler_context.agent_name, event);
            }
            Some(AdvanceOutcome::ChannelDropped) => {
                request.i_batch = None;
                request.phase = ContinuousBatchRequestPhase::Completed;
            }
        }
    }
}
