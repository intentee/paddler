use llama_cpp_bindings::error::SamplingError;
use llama_cpp_bindings::sampling::LlamaSampler;

use paddler_inference_parameters::sampling_parameters::SamplingParameters;

pub struct SamplerChainFactory {
    pub sampling_parameters: SamplingParameters,
    pub n_vocab: i32,
}

impl SamplerChainFactory {
    pub fn create(&self, seed: u32) -> Result<LlamaSampler, SamplingError> {
        let sampling_parameters = &self.sampling_parameters;
        let samplers = [
            LlamaSampler::penalties(
                self.n_vocab,
                sampling_parameters.penalty_last_n,
                sampling_parameters.penalty_repeat,
                sampling_parameters.penalty_frequency,
                sampling_parameters.penalty_presence,
            ),
            LlamaSampler::top_k(sampling_parameters.top_k),
            LlamaSampler::top_p(sampling_parameters.top_p, 0),
            LlamaSampler::min_p(sampling_parameters.min_p, 0),
            LlamaSampler::temp(sampling_parameters.temperature),
            LlamaSampler::dist(seed),
        ];

        LlamaSampler::chain_simple(samplers.into_iter().collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;
    use llama_cpp_bindings::token::data::LlamaTokenData;
    use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

    use paddler_inference_parameters::sampling_parameters::SamplingParameters;

    use super::SamplerChainFactory;

    #[test]
    fn chain_honors_configured_top_k() {
        let sampling_parameters = SamplingParameters {
            top_k: 1,
            ..SamplingParameters::default()
        };
        let sampler_chain = SamplerChainFactory {
            sampling_parameters,
            n_vocab: 3,
        }
        .create(7)
        .unwrap();
        let mut candidates = LlamaTokenDataArray::from_iter(
            [
                LlamaTokenData::new(LlamaToken(0), 0.5, 0.0),
                LlamaTokenData::new(LlamaToken(1), 3.0, 0.0),
                LlamaTokenData::new(LlamaToken(2), 1.0, 0.0),
            ],
            false,
        );

        candidates.apply_sampler(&sampler_chain).unwrap();

        assert_eq!(candidates.selected_token(), Some(LlamaToken(1)));
    }
}
