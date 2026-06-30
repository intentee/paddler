use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::token::LlamaToken;
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

fn flush_tool_call_pipeline_on_completion(
    agent_name: Option<&str>,
    request: &mut ContinuousBatchActiveRequest,
    flush_context: &str,
) -> Option<AdvanceOutcome> {
    if let Some(pipeline) = request.tool_call_pipeline.as_mut()
        && !pipeline.buffer_is_empty()
        && let Some(event) = pipeline.finalize_to_generated_event()
        && request.generated_tokens_tx.send(event).is_err()
    {
        warn!(
            "{agent_name:?}: sequence {} client disconnected (receiver dropped) during {flush_context}",
            request.state.sequence_id
        );

        return Some(AdvanceOutcome::ChannelDropped);
    }

    None
}

fn channel_dropped(agent_name: Option<&str>, sequence_id: u16) -> AdvanceOutcome {
    warn!("{agent_name:?}: sequence {sequence_id} client disconnected (receiver dropped)");

    AdvanceOutcome::ChannelDropped
}

pub fn completion_from_sample_outcome(
    outcome: SampleOutcome,
) -> Result<LlamaToken, GeneratedTokenResult> {
    match outcome {
        SampleOutcome::Sampled(token) => Ok(token),
        SampleOutcome::AllCandidatesEliminated => Err(GeneratedTokenResult::SamplerError(
            "all token candidates were eliminated during sampling".to_owned(),
        )),
        SampleOutcome::GrammarRejected(message) => {
            Err(GeneratedTokenResult::GrammarRejectedModelOutput(message))
        }
        SampleOutcome::Failed(message) => Err(GeneratedTokenResult::SamplerError(message)),
    }
}

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

        let sample_outcome = (SampleTokenPhase {
            context: self.llama_context,
        })
        .run(request, batch_index);
        let raw_token = match completion_from_sample_outcome(sample_outcome) {
            Ok(token) => token,
            Err(event) => {
                error!(
                    "{:?}: sequence {} sampling terminated early: {event:?}",
                    self.scheduler_context.agent_name, request.state.sequence_id
                );
                return Some(AdvanceOutcome::Completed(event));
            }
        };

        let classified_outcomes = match classify_token_phase::run(request, raw_token) {
            Ok(outcomes) => outcomes,
            Err(error) => {
                error!(
                    "{:?}: sequence {} token classification failed: {error:#}",
                    self.scheduler_context.agent_name, request.state.sequence_id
                );

                return Some(AdvanceOutcome::Completed(
                    GeneratedTokenResult::DetokenizationFailed(error.to_string()),
                ));
            }
        };

        let completion_phase = CompletionCheckPhase {
            model: &self.scheduler_context.model,
        };

        let raw_as_sampled = SampledToken::Content(raw_token);
        if matches!(
            completion_phase.run(request, &raw_as_sampled),
            CompletionCheckOutcome::ReachedEog
        ) {
            if let Some(channel_dropped) = flush_tool_call_pipeline_on_completion(
                self.scheduler_context.agent_name.as_deref(),
                request,
                "EOG tool-call flush",
            ) {
                return Some(channel_dropped);
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
                    return Some(channel_dropped(
                        self.scheduler_context.agent_name.as_deref(),
                        request.state.sequence_id,
                    ));
                }
            }

            if let Some(event) =
                tool_call_pass::run(request.tool_call_pipeline.as_mut(), classified)
                && request.generated_tokens_tx.send(event).is_err()
            {
                return Some(channel_dropped(
                    self.scheduler_context.agent_name.as_deref(),
                    request.state.sequence_id,
                ));
            }
        }

        match completion_phase.run(request, &raw_as_sampled) {
            CompletionCheckOutcome::ReachedEog | CompletionCheckOutcome::ReachedMaxTokens => {
                if let Some(channel_dropped) = flush_tool_call_pipeline_on_completion(
                    self.scheduler_context.agent_name.as_deref(),
                    request,
                    "tool-call EOG flush",
                ) {
                    return Some(channel_dropped);
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

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::token::LlamaToken;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    use super::channel_dropped;
    use super::completion_from_sample_outcome;
    use crate::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
    use crate::continuous_batch_scheduler::sample_outcome::SampleOutcome;

    #[test]
    fn channel_dropped_reports_the_dropped_channel_outcome() {
        let outcome = channel_dropped(Some("agent"), 5);

        assert_eq!(
            discriminant(&outcome),
            discriminant(&AdvanceOutcome::ChannelDropped)
        );
    }

    #[test]
    fn sampled_outcome_yields_the_token() {
        let result = completion_from_sample_outcome(SampleOutcome::Sampled(LlamaToken::new(7)));

        assert!(matches!(result, Ok(token) if token == LlamaToken::new(7)));
    }

    #[test]
    fn all_candidates_eliminated_yields_sampler_error() {
        let result = completion_from_sample_outcome(SampleOutcome::AllCandidatesEliminated);

        assert!(matches!(
            result,
            Err(ref generated)
                if discriminant(generated)
                    == discriminant(&GeneratedTokenResult::SamplerError(String::new()))
        ));
    }

    #[test]
    fn grammar_rejected_yields_grammar_rejected_model_output() {
        let result =
            completion_from_sample_outcome(SampleOutcome::GrammarRejected("nope".to_owned()));

        assert!(matches!(
            result,
            Err(GeneratedTokenResult::GrammarRejectedModelOutput(message)) if message == "nope"
        ));
    }

    #[test]
    fn failed_outcome_yields_sampler_error() {
        let result = completion_from_sample_outcome(SampleOutcome::Failed("boom".to_owned()));

        assert!(matches!(
            result,
            Err(GeneratedTokenResult::SamplerError(message)) if message == "boom"
        ));
    }
}
