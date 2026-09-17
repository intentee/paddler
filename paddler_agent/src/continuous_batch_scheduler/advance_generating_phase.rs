use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::TokenUsage;
use llama_cpp_bindings::context::LlamaContext;
use log::error;
use log::warn;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use tokio::sync::mpsc;

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
use crate::tool_call_pipeline::ToolCallPipeline;

fn warn_channel_dropped(agent_name: Option<&str>, sequence_id: i32) {
    warn!("{agent_name:?}: sequence {sequence_id} client disconnected (receiver dropped)");
}

fn emit_all(
    agent_name: Option<&str>,
    sequence_id: i32,
    tool_call_pipeline: Option<&mut ToolCallPipeline>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    classified_tokens: &[ClassifiedToken],
) -> Option<AdvanceOutcome> {
    match emit_classified_tokens::run(tool_call_pipeline, generated_tokens_tx, classified_tokens) {
        ClientStreamStatus::Open => None,
        ClientStreamStatus::Dropped => {
            warn_channel_dropped(agent_name, sequence_id);

            Some(AdvanceOutcome::ChannelDropped)
        }
    }
}

fn deliver_final_tokens(
    agent_name: Option<&str>,
    sequence_id: i32,
    mut tool_call_pipeline: Option<&mut ToolCallPipeline>,
    generated_tokens_tx: &mpsc::UnboundedSender<GeneratedTokenResult>,
    usage: TokenUsage,
    flushed_tokens: &[ClassifiedToken],
) -> AdvanceOutcome {
    if let Some(outcome) = emit_all(
        agent_name,
        sequence_id,
        tool_call_pipeline.as_deref_mut(),
        generated_tokens_tx,
        flushed_tokens,
    ) {
        return outcome;
    }

    if let Some(pipeline) = tool_call_pipeline
        && !pipeline.buffer_is_empty()
        && let Some(event) = pipeline.finalize_to_generated_event()
        && generated_tokens_tx.send(event).is_err()
    {
        warn_channel_dropped(agent_name, sequence_id);

        return AdvanceOutcome::ChannelDropped;
    }

    AdvanceOutcome::Completed(GeneratedTokenResult::Done(GenerationSummary { usage }))
}

fn sampling_failed(
    agent_name: Option<&str>,
    sequence_id: i32,
    failure_result: GeneratedTokenResult,
) -> AdvanceOutcome {
    error!("{agent_name:?}: sequence {sequence_id} sampling failed: {failure_result:?}");

    AdvanceOutcome::Completed(failure_result)
}

fn classification_failed(
    agent_name: Option<&str>,
    sequence_id: i32,
    error: &anyhow::Error,
) -> AdvanceOutcome {
    error!("{agent_name:?}: sequence {sequence_id} token classification failed: {error:#}");

    AdvanceOutcome::Completed(GeneratedTokenResult::DetokenizationFailed(
        error.to_string(),
    ))
}

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

    fn finish(&self, request: &mut ContinuousBatchActiveRequest) -> AdvanceOutcome {
        let flushed_tokens = classify_token_phase::flush(request);
        let usage = *request.token_classifier.usage();

        deliver_final_tokens(
            self.scheduler_context.agent_name.as_deref(),
            request.sequence_id_guard.sequence_id(),
            request.tool_call_pipeline.as_mut(),
            &request.generated_tokens_tx,
            usage,
            &flushed_tokens,
        )
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
                return Some(sampling_failed(
                    self.scheduler_context.agent_name.as_deref(),
                    request.sequence_id_guard.sequence_id(),
                    failure_result,
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
                return Some(classification_failed(
                    self.scheduler_context.agent_name.as_deref(),
                    request.sequence_id_guard.sequence_id(),
                    &error,
                ));
            }
        };

        if let Some(outcome) = emit_all(
            self.scheduler_context.agent_name.as_deref(),
            request.sequence_id_guard.sequence_id(),
            request.tool_call_pipeline.as_mut(),
            &request.generated_tokens_tx,
            &classified_tokens,
        ) {
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

    use anyhow::anyhow;
    use llama_cpp_bindings::TokenUsage;
    use tokio::sync::mpsc;

    use super::apply_outcome;
    use super::classification_failed;
    use super::deliver_final_tokens;
    use super::emit_all;
    use super::sampling_failed;
    use crate::continuous_batch_generating_state::ContinuousBatchGeneratingState;
    use crate::continuous_batch_ingesting_state::ContinuousBatchIngestingState;
    use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
    use crate::continuous_batch_request_state::ContinuousBatchRequestState;
    use crate::continuous_batch_scheduler::advance_outcome::AdvanceOutcome;
    use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;
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

    fn content_token(piece: &str) -> ClassifiedToken {
        ClassifiedToken {
            sampled_token: SampledToken::Content(LlamaToken::new(1)),
            was_in_tool_call: false,
            is_in_tool_call: false,
            visible_piece: piece.to_owned(),
            raw_piece: piece.to_owned(),
        }
    }

    #[test]
    fn emitting_to_a_listening_client_does_not_end_the_request() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();

        assert!(
            emit_all(
                Some("test-agent"),
                3,
                None,
                &generated_tokens_tx,
                &[content_token("hi")],
            )
            .is_none()
        );
        assert_eq!(
            generated_tokens_rx
                .try_recv()
                .unwrap()
                .token_text()
                .unwrap(),
            "hi"
        );
    }

    #[test]
    fn emitting_to_a_disconnected_client_ends_the_request_without_emitting() {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        assert!(matches!(
            emit_all(
                Some("test-agent"),
                3,
                None,
                &generated_tokens_tx,
                &[content_token("hi")],
            ),
            Some(AdvanceOutcome::ChannelDropped)
        ));
    }

    #[test]
    fn delivering_the_final_tokens_completes_the_request_with_its_usage() {
        let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
        let usage = TokenUsage {
            content_tokens: 2,
            ..TokenUsage::new()
        };

        let outcome = deliver_final_tokens(
            Some("test-agent"),
            3,
            None,
            &generated_tokens_tx,
            usage,
            &[content_token("bye")],
        );

        assert!(matches!(
            outcome,
            AdvanceOutcome::Completed(GeneratedTokenResult::Done(summary))
                if summary.usage.content_tokens == 2
        ));
        assert_eq!(
            generated_tokens_rx
                .try_recv()
                .unwrap()
                .token_text()
                .unwrap(),
            "bye"
        );
    }

    #[test]
    fn delivering_the_final_tokens_to_a_disconnected_client_reports_the_dropped_stream() {
        let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();

        drop(generated_tokens_rx);

        assert!(matches!(
            deliver_final_tokens(
                Some("test-agent"),
                3,
                None,
                &generated_tokens_tx,
                TokenUsage::new(),
                &[content_token("bye")],
            ),
            AdvanceOutcome::ChannelDropped
        ));
    }

    #[test]
    fn a_sampling_failure_completes_the_request_with_that_failure() {
        assert!(matches!(
            sampling_failed(
                Some("test-agent"),
                3,
                GeneratedTokenResult::SamplerError("no candidates".to_owned()),
            ),
            AdvanceOutcome::Completed(GeneratedTokenResult::SamplerError(message))
                if message == "no candidates"
        ));
    }

    #[test]
    fn a_classification_failure_completes_the_request_as_a_detokenization_failure() {
        assert!(matches!(
            classification_failed(Some("test-agent"), 3, &anyhow!("bad utf8")),
            AdvanceOutcome::Completed(GeneratedTokenResult::DetokenizationFailed(message))
                if message == "bad utf8"
        ));
    }
}
