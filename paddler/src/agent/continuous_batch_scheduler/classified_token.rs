use llama_cpp_bindings::SampledToken;

pub struct ClassifiedToken {
    pub sampled_token: SampledToken,
    pub was_in_tool_call: bool,
    pub is_in_tool_call: bool,
}
