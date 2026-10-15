use llama_cpp_bindings::GenerationProgress;
use llama_cpp_bindings::TokenUsage;

use crate::continuous_batch_request_state::ContinuousBatchRequestState;
use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;

pub struct CompletionCheckPhase<'request> {
    pub request_state: &'request ContinuousBatchRequestState,
    pub sequence_context_size: u32,
    pub usage: &'request TokenUsage,
}

impl CompletionCheckPhase<'_> {
    #[must_use]
    pub fn run(self, progress: GenerationProgress) -> CompletionCheckOutcome {
        match progress {
            GenerationProgress::Ended => CompletionCheckOutcome::ReachedEndOfGeneration,
            GenerationProgress::Continues if self.reached_max_tokens() => {
                CompletionCheckOutcome::ReachedMaxTokens
            }
            GenerationProgress::Continues if self.filled_sequence_context() => {
                CompletionCheckOutcome::ReachedContextLimit
            }
            GenerationProgress::Continues => CompletionCheckOutcome::Continue,
        }
    }

    fn reached_max_tokens(&self) -> bool {
        self.request_state
            .sampled_tokens
            .saturating_sub(self.usage.tool_call_tokens)
            >= u64::from(self.request_state.max_tokens.get())
    }

    fn filled_sequence_context(&self) -> bool {
        i64::from(self.request_state.current_token_position)
            >= i64::from(self.sequence_context_size)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use llama_cpp_bindings::GenerationProgress;
    use llama_cpp_bindings::SampledTokenSection;
    use llama_cpp_bindings::TokenUsage;

    use super::CompletionCheckPhase;
    use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
    use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
    use crate::continuous_batch_request_state::ContinuousBatchRequestState;
    use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;

    const SEQUENCE_CONTEXT_SIZE: u32 = 16;

    fn generating_state(
        sampled_tokens: u64,
        current_token_position: i32,
    ) -> ContinuousBatchRequestState {
        ContinuousBatchRequestState {
            current_token_position,
            last_outcome_section: SampledTokenSection::Content,
            max_tokens: NonZeroU32::new(4).unwrap(),
            phase: ContinuousBatchRequestPhase::Generating(
                ContinuousBatchGenerationStep::ReadyToSample { batch_position: 0 },
            ),
            prompt_tokens: Vec::new(),
            prompt_tokens_ingested: 0,
            sampled_tokens,
        }
    }

    fn check(
        progress: GenerationProgress,
        request_state: &ContinuousBatchRequestState,
        usage: &TokenUsage,
    ) -> CompletionCheckOutcome {
        CompletionCheckPhase {
            request_state,
            sequence_context_size: SEQUENCE_CONTEXT_SIZE,
            usage,
        }
        .run(progress)
    }

    #[test]
    fn end_of_generation_completes_regardless_of_the_budget() {
        assert_eq!(
            check(
                GenerationProgress::Ended,
                &generating_state(1, 1),
                &TokenUsage::new()
            ),
            CompletionCheckOutcome::ReachedEndOfGeneration
        );
    }

    #[test]
    fn tokens_held_back_from_usage_count_toward_max_tokens() {
        let usage = TokenUsage {
            content_tokens: 1,
            ..TokenUsage::new()
        };

        assert_eq!(
            check(
                GenerationProgress::Continues,
                &generating_state(4, 4),
                &usage
            ),
            CompletionCheckOutcome::ReachedMaxTokens
        );
    }

    #[test]
    fn finalized_tool_call_tokens_do_not_count_toward_max_tokens() {
        let usage = TokenUsage {
            tool_call_tokens: 3,
            ..TokenUsage::new()
        };

        assert_eq!(
            check(
                GenerationProgress::Continues,
                &generating_state(5, 5),
                &usage
            ),
            CompletionCheckOutcome::Continue
        );
    }

    #[test]
    fn a_sequence_that_filled_its_context_reaches_the_context_limit() {
        assert_eq!(
            check(
                GenerationProgress::Continues,
                &generating_state(1, 16),
                &TokenUsage::new()
            ),
            CompletionCheckOutcome::ReachedContextLimit
        );
    }
}
