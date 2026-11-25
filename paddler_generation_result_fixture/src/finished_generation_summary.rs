use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::token_usage::TokenUsage;

#[must_use]
pub fn finished_generation_summary(finish: GenerationFinish) -> GenerationSummary {
    GenerationSummary {
        finish,
        usage: TokenUsage {
            prompt: 7,
            content: 4,
            reasoning: 1,
            ..TokenUsage::default()
        },
    }
}
