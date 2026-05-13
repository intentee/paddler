use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::TokenUsage;
use llama_cpp_bindings::model::LlamaModel;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_scheduler::completion_check_outcome::CompletionCheckOutcome;

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

        #[expect(
            clippy::cast_sign_loss,
            reason = "max_tokens is non-negative by API contract"
        )]
        let max_tokens_u64 = request.max_tokens as u64;

        if completion_token_count(request.token_classifier.usage()) >= max_tokens_u64 {
            CompletionCheckOutcome::ReachedMaxTokens
        } else {
            CompletionCheckOutcome::Continue
        }
    }
}

const fn completion_token_count(usage: &TokenUsage) -> u64 {
    usage.content_tokens + usage.reasoning_tokens + usage.undeterminable_tokens
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::TokenUsage;

    use super::completion_token_count;

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
