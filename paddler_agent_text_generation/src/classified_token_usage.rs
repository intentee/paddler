use llama_cpp_bindings_types::token_usage::TokenUsage as ClassifierTokenUsage;

use paddler_messaging::token_usage::TokenUsage;

pub struct ClassifiedTokenUsage<'usage>(pub &'usage ClassifierTokenUsage);

impl ClassifiedTokenUsage<'_> {
    #[must_use]
    pub const fn to_token_usage(&self) -> TokenUsage {
        let ClassifierTokenUsage {
            prompt_tokens,
            cached_prompt_tokens,
            input_image_tokens,
            input_audio_tokens,
            content_tokens,
            reasoning_tokens,
            tool_call_tokens,
            undeterminable_tokens,
        } = *self.0;

        TokenUsage {
            prompt: prompt_tokens,
            cached_prompt: cached_prompt_tokens,
            input_image: input_image_tokens,
            input_audio: input_audio_tokens,
            content: content_tokens,
            reasoning: reasoning_tokens,
            tool_call: tool_call_tokens,
            undeterminable: undeterminable_tokens,
        }
    }
}
