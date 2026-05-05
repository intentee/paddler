use llama_cpp_bindings::SampledToken;
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

        let usage = request.token_classifier.usage();
        let completion_so_far =
            usage.content_tokens + usage.reasoning_tokens + usage.undeterminable_tokens;

        #[expect(
            clippy::cast_sign_loss,
            reason = "max_tokens is non-negative by API contract"
        )]
        let max_tokens_u64 = request.max_tokens as u64;

        if completion_so_far >= max_tokens_u64 {
            CompletionCheckOutcome::ReachedMaxTokens
        } else {
            CompletionCheckOutcome::Continue
        }
    }
}
