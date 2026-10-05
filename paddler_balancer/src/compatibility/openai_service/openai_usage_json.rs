use llama_cpp_bindings_types::TokenUsage;
use serde_json::Value;
use serde_json::json;

#[must_use]
pub fn openai_usage_json(usage: &TokenUsage) -> Value {
    json!({
        "prompt_tokens": usage.prompt_tokens,
        "completion_tokens": usage.completion_tokens(),
        "total_tokens": usage.total_tokens(),
        "prompt_tokens_details": {
            "cached_tokens": usage.cached_prompt_tokens,
            "audio_tokens": usage.input_audio_tokens,
        },
        "completion_tokens_details": {
            "reasoning_tokens": usage.reasoning_tokens,
        }
    })
}
