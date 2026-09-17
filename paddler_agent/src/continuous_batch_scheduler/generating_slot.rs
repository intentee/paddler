use llama_cpp_bindings::SampledToken;

pub struct GeneratingSlot {
    pub request_index: usize,
    pub sampled_token: SampledToken,
    pub position: i32,
    pub sequence_id: i32,
}
