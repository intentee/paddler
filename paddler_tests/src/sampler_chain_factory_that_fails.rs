use llama_cpp_bindings::error::SamplingError;
use llama_cpp_bindings::sampling::LlamaSampler;

use paddler_agent_text_generation::creates_sampler_chain::CreatesSamplerChain;

pub struct SamplerChainFactoryThatFails;

impl CreatesSamplerChain for SamplerChainFactoryThatFails {
    fn create(&self, _seed: u32) -> Result<LlamaSampler, SamplingError> {
        Err(SamplingError::SamplerUnavailable { sampler: "chain" })
    }
}
