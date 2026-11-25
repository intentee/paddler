use std::mem::Discriminant;
use std::mem::discriminant;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::token_usage::TokenUsage;

#[must_use]
pub fn generation_done_discriminant() -> Discriminant<GeneratedTokenResult> {
    discriminant(&GeneratedTokenResult::Done(GenerationSummary {
        finish: GenerationFinish::EndOfGeneration,
        usage: TokenUsage::default(),
    }))
}
