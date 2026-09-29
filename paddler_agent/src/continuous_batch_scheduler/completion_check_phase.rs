use std::num::NonZeroU32;

use llama_cpp_bindings::GenerationProgress;
use llama_cpp_bindings::TokenUsage;

use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;

fn max_tokens_outcome(
    max_tokens: NonZeroU32,
    sampled_tokens: u64,
    usage: &TokenUsage,
) -> CompletionCheckOutcome {
    if sampled_tokens.saturating_sub(usage.tool_call_tokens) >= u64::from(max_tokens.get()) {
        CompletionCheckOutcome::ReachedMaxTokens
    } else {
        CompletionCheckOutcome::Continue
    }
}

#[must_use]
pub fn run(
    progress: GenerationProgress,
    max_tokens: NonZeroU32,
    sampled_tokens: u64,
    usage: &TokenUsage,
) -> CompletionCheckOutcome {
    match progress {
        GenerationProgress::Ended => CompletionCheckOutcome::ReachedEndOfGeneration,
        GenerationProgress::Continues => max_tokens_outcome(max_tokens, sampled_tokens, usage),
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use llama_cpp_bindings::GenerationProgress;
    use llama_cpp_bindings::TokenUsage;

    use super::run;

    const FOUR_TOKENS: NonZeroU32 = NonZeroU32::new(4).unwrap();
    use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;

    #[test]
    fn end_of_generation_completes_regardless_of_the_budget() {
        assert!(matches!(
            run(
                GenerationProgress::Ended,
                FOUR_TOKENS,
                1,
                &TokenUsage::new()
            ),
            CompletionCheckOutcome::ReachedEndOfGeneration
        ));
    }

    #[test]
    fn tokens_held_back_from_usage_count_toward_max_tokens() {
        let usage = TokenUsage {
            content_tokens: 1,
            ..TokenUsage::new()
        };

        assert!(matches!(
            run(GenerationProgress::Continues, FOUR_TOKENS, 4, &usage),
            CompletionCheckOutcome::ReachedMaxTokens
        ));
    }

    #[test]
    fn finalized_tool_call_tokens_do_not_count_toward_max_tokens() {
        let usage = TokenUsage {
            tool_call_tokens: 3,
            ..TokenUsage::new()
        };

        assert!(matches!(
            run(GenerationProgress::Continues, FOUR_TOKENS, 5, &usage),
            CompletionCheckOutcome::Continue
        ));
    }
}
