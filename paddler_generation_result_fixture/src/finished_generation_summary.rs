use llama_cpp_bindings_types::token_usage::TokenUsage;

use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;

#[must_use]
pub fn finished_generation_summary(finish: GenerationFinish) -> GenerationSummary {
    GenerationSummary {
        finish,
        usage: TokenUsage {
            prompt_tokens: 7,
            content_tokens: 4,
            reasoning_tokens: 1,
            ..TokenUsage::default()
        },
    }
}
