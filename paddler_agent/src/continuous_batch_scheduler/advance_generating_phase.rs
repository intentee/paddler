use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use log::error;
use log::warn;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_generating_state::ContinuousBatchGeneratingState;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_request_state::ContinuousBatchRequestState;
use crate::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
use crate::continuous_batch_scheduler::classify_token_phase;
use crate::continuous_batch_scheduler::client_stream_status::ClientStreamStatus;
use crate::continuous_batch_scheduler::completion_check_phase::CompletionCheckPhase;
use crate::continuous_batch_scheduler::emit_classified_tokens;
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

            apply_outcome(&mut request.state, outcome);
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

        let sample_outcome = (SampleTokenPhase {
            context: self.llama_context,
        })
        .run(request, batch_index);

        let raw_token = match sample_outcome.into_sampled_token() {
            Ok(token) => token,
            Err(failure_result) => {
                error!(
                    "{:?}: sequence {} sampling failed: {failure_result:?}",
                    self.scheduler_context.agent_name,
                    request.sequence_id_guard.sequence_id()
                );

                return Some(AdvanceOutcome::Completed(failure_result));
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
}

fn apply_outcome(state: &mut ContinuousBatchRequestState, outcome: Option<AdvanceOutcome>) {
    match outcome {
        None => {}
        Some(AdvanceOutcome::SampledAndStored(sampled_token)) => {
            state.store_pending_token(sampled_token);
        }
        Some(AdvanceOutcome::Completed(event)) => {
            state.mark_completed(ContinuousBatchTerminalOutcome::EmitToClient(event));
        }
        Some(AdvanceOutcome::ChannelDropped) => {
            state.mark_completed(ContinuousBatchTerminalOutcome::EmitNothing);
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;

    use super::apply_outcome;
    use crate::continuous_batch_generating_state::ContinuousBatchGeneratingState;
    use crate::continuous_batch_ingesting_state::ContinuousBatchIngestingState;
    use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
    use crate::continuous_batch_request_state::ContinuousBatchRequestState;
    use crate::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
    use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;

    fn generating_state() -> ContinuousBatchRequestState {
        ContinuousBatchRequestState {
            current_token_position: 0,
            max_tokens: 8,
            phase: ContinuousBatchRequestPhase::Generating(
                ContinuousBatchGeneratingState::AwaitingSample { batch_index: 0 },
            ),
        }
    }

    #[test]
    fn a_request_that_did_not_advance_keeps_waiting_for_its_sample() {
        let mut state = generating_state();

        apply_outcome(&mut state, None);

        assert!(matches!(
            state.phase,
            ContinuousBatchRequestPhase::Generating(
                ContinuousBatchGeneratingState::AwaitingSample { batch_index: 0 }
            )
        ));
    }

    #[test]
    fn a_sampled_token_waits_for_a_slot_in_the_next_batch() {
        let mut state = generating_state();

        apply_outcome(
            &mut state,
            Some(AdvanceOutcome::SampledAndStored(SampledToken::Content(
                LlamaToken::new(7),
            ))),
        );

        assert!(matches!(
            state.phase,
            ContinuousBatchRequestPhase::Generating(
                ContinuousBatchGeneratingState::AwaitingBatchSlot { .. }
            )
        ));
    }

    #[test]
    fn a_completed_request_delivers_its_event_to_the_client() {
        let mut state = generating_state();

        apply_outcome(
            &mut state,
            Some(AdvanceOutcome::Completed(
                GeneratedTokenResult::SamplerError("boom".to_owned()),
            )),
        );

        assert!(matches!(
            state.into_terminal_outcome(),
            ContinuousBatchTerminalOutcome::EmitToClient(GeneratedTokenResult::SamplerError(
                message
            )) if message == "boom"
        ));
    }

    #[test]
    fn a_dropped_client_stream_ends_the_request_without_emitting_anything() {
        let mut state = ContinuousBatchRequestState {
            current_token_position: 0,
            max_tokens: 8,
            phase: ContinuousBatchRequestPhase::Ingesting(ContinuousBatchIngestingState::new(
                Vec::new(),
            )),
        };

        apply_outcome(&mut state, Some(AdvanceOutcome::ChannelDropped));

        assert!(matches!(
            state.into_terminal_outcome(),
            ContinuousBatchTerminalOutcome::EmitNothing
        ));
    }
}
