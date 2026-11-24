use llama_cpp_token_usage::token_usage::TokenUsage as ClassifierTokenUsage;

use paddler_messaging::token_usage::TokenUsage;

pub struct ClassifiedTokenUsage<'usage>(pub &'usage ClassifierTokenUsage);

impl ClassifiedTokenUsage<'_> {
    #[must_use]
    pub const fn to_token_usage(&self) -> TokenUsage {
        let ClassifierTokenUsage {
            prompt,
            cached_prompt,
            input_image,
            input_audio,
            content,
            reasoning,
            tool_call,
            undeterminable,
        } = *self.0;

        TokenUsage {
            prompt,
            cached_prompt,
            input_image,
            input_audio,
            content,
            reasoning,
            tool_call,
            undeterminable,
        }
    }
}
