use llama_cpp_bindings::error::SamplingError;
use llama_cpp_bindings::sampling::LlamaSampler;

pub trait CreatesSamplerChain {
    fn create(&self, seed: u32) -> Result<LlamaSampler, SamplingError>;
}
