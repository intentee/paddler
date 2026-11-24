use llama_cpp_model::sampled_token::SampledToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;

pub enum AdvanceOutcome {
    SampledAndStored(SampledToken),
    Completed(GeneratedTokenResult),
    ChannelDropped,
}
