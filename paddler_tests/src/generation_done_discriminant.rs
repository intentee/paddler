use std::mem::Discriminant;
use std::mem::discriminant;

use llama_cpp_bindings::TokenUsage;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;

#[must_use]
pub const fn generation_done_discriminant() -> Discriminant<GeneratedTokenResult> {
    discriminant(&GeneratedTokenResult::Done(GenerationSummary {
        finish: GenerationFinish::EndOfGeneration,
        usage: TokenUsage::new(),
    }))
}
