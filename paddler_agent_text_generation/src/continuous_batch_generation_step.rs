use llama_cpp_bindings::sampled_token::SampledToken;

#[derive(Debug)]
pub enum ContinuousBatchGenerationStep {
    AwaitingDecode(SampledToken),
    ReadyToSample { batch_position: i32 },
}
