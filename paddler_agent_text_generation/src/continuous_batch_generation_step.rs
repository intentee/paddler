use llama_cpp_model::sampled_token::SampledToken;

#[derive(Debug)]
pub enum ContinuousBatchGenerationStep {
    AwaitingDecode(SampledToken),
    ReadyToSample { batch_position: i32 },
}
