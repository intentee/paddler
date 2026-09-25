use llama_cpp_bindings::error::SamplingError;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_messaging::inference_parameters::InferenceParameters;

pub struct SamplerChainFactory<'parameters> {
    pub inference_parameters: &'parameters InferenceParameters,
    pub n_vocab: i32,
}

impl SamplerChainFactory<'_> {
    pub fn create(&self, seed: u32) -> Result<LlamaSampler, SamplingError> {
        let inference_parameters = self.inference_parameters;
        let samplers = [
            LlamaSampler::penalties(
                self.n_vocab,
                inference_parameters.penalty_last_n,
                inference_parameters.penalty_repeat,
                inference_parameters.penalty_frequency,
                inference_parameters.penalty_presence,
            ),
            LlamaSampler::top_k(inference_parameters.top_k),
            LlamaSampler::top_p(inference_parameters.top_p, 0),
            LlamaSampler::min_p(inference_parameters.min_p, 0),
            LlamaSampler::temp(inference_parameters.temperature),
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
    use paddler_messaging::inference_parameters::InferenceParameters;

    use super::SamplerChainFactory;

    #[test]
    fn chain_honors_configured_top_k() {
        let inference_parameters = InferenceParameters {
            top_k: 1,
            ..InferenceParameters::default()
        };
        let sampler_chain = SamplerChainFactory {
            inference_parameters: &inference_parameters,
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
