use llama_cpp_bindings::SampledToken;

#[derive(Debug)]
pub enum ContinuousBatchGeneratingState {
    AwaitingSample { batch_index: i32 },
    AwaitingBatchSlot { sampled_token: SampledToken },
}
