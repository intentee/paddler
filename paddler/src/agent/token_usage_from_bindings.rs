use llama_cpp_bindings::TokenUsage as BindingsTokenUsage;
use paddler_types::token_usage::TokenUsage;

#[must_use]
pub const fn token_usage_from_bindings(usage: &BindingsTokenUsage) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage.prompt_tokens(),
        cached_prompt_tokens: usage.cached_prompt_tokens(),
        input_image_tokens: usage.input_image_tokens(),
        input_audio_tokens: usage.input_audio_tokens(),
        content_tokens: usage.content_tokens(),
        reasoning_tokens: usage.reasoning_tokens(),
        tool_call_tokens: usage.tool_call_tokens(),
        undeterminable_tokens: usage.undeterminable_tokens(),
    }
}
