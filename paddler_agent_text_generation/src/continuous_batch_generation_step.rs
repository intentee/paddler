use llama_cpp_bindings::SampledToken;

#[derive(Debug)]
pub enum ContinuousBatchGenerationStep {
    AwaitingDecode(SampledToken),
    ReadyToSample { batch_position: i32 },
}
