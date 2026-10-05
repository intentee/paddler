use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::TokenUsage;
use llama_cpp_bindings::model::LlamaModel;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;

pub struct CompletionCheckPhase<'model> {
    pub model: &'model LlamaModel,
}

impl CompletionCheckPhase<'_> {
    #[must_use]
    pub fn run(
        &self,
        request: &ContinuousBatchActiveRequest,
        sampled_token: &SampledToken,
    ) -> CompletionCheckOutcome {
        if self.model.is_eog_token(sampled_token) {
            return CompletionCheckOutcome::ReachedEog;
        }

        max_tokens_outcome(request.state.max_tokens, request.token_classifier.usage())
    }
}

fn max_tokens_outcome(max_tokens: i32, usage: &TokenUsage) -> CompletionCheckOutcome {
    let Ok(max_tokens_u64) = u64::try_from(max_tokens) else {
        return CompletionCheckOutcome::ReachedMaxTokens;
    };

    if completion_token_count(usage) >= max_tokens_u64 {
        CompletionCheckOutcome::ReachedMaxTokens
    } else {
        CompletionCheckOutcome::Continue
    }
}

const fn completion_token_count(usage: &TokenUsage) -> u64 {
    usage.content_tokens + usage.reasoning_tokens + usage.undeterminable_tokens
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::TokenUsage;

    use super::completion_token_count;
    use super::max_tokens_outcome;
    use crate::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;

    #[test]
    fn negative_max_tokens_reaches_max_tokens() {
        let usage = TokenUsage::new();

        assert!(matches!(
            max_tokens_outcome(-1, &usage),
            CompletionCheckOutcome::ReachedMaxTokens
        ));
    }

    #[test]
    fn reaching_the_max_token_budget_reports_reached_max_tokens() {
        let usage = TokenUsage {
            content_tokens: 4,
            ..TokenUsage::new()
        };

        assert!(matches!(
            max_tokens_outcome(4, &usage),
            CompletionCheckOutcome::ReachedMaxTokens
        ));
    }

    #[test]
    fn staying_under_the_max_token_budget_continues() {
        let usage = TokenUsage {
            content_tokens: 3,
            ..TokenUsage::new()
        };

        assert!(matches!(
            max_tokens_outcome(4, &usage),
            CompletionCheckOutcome::Continue
        ));
    }

    #[test]
    fn completion_token_count_sums_content_reasoning_and_undeterminable() {
        let usage = TokenUsage {
            content_tokens: 5,
            reasoning_tokens: 3,
            undeterminable_tokens: 2,
            ..TokenUsage::new()
        };

        assert_eq!(completion_token_count(&usage), 10);
    }

    #[test]
    fn completion_token_count_excludes_prompt_and_cached_prompt_tokens() {
        let usage = TokenUsage {
            prompt_tokens: 100,
            cached_prompt_tokens: 50,
            input_image_tokens: 20,
            input_audio_tokens: 10,
            content_tokens: 4,
            reasoning_tokens: 0,
            tool_call_tokens: 0,
            undeterminable_tokens: 0,
        };

        assert_eq!(completion_token_count(&usage), 4);
    }

    #[test]
    fn completion_token_count_excludes_tool_call_tokens() {
        let usage = TokenUsage {
            content_tokens: 1,
            reasoning_tokens: 0,
            tool_call_tokens: 99,
            undeterminable_tokens: 0,
            ..TokenUsage::new()
        };

        assert_eq!(completion_token_count(&usage), 1);
    }
}
