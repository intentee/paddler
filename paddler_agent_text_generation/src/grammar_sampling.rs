use llama_cpp_model::llama_token::LlamaToken;
use llama_cpp_model::llama_token_data_array::LlamaTokenDataArray;
use llama_cpp_sampler::llama_sampler::LlamaSampler;
use llama_cpp_sampler::sampler_error::SamplerError;

pub enum GrammarSampling {
    Constrained(LlamaSampler),
    Unconstrained,
}

impl GrammarSampling {
    /// # Errors
    ///
    /// Returns the [`SamplerError`] of the grammar sampler when the grammar rejects the token.
    pub fn accept(&mut self, token: LlamaToken) -> Result<(), SamplerError> {
        match self {
            Self::Constrained(grammar_sampler) => grammar_sampler.accept(token),
            Self::Unconstrained => Ok(()),
        }
    }

    /// # Errors
    ///
    /// Returns the [`SamplerError`] of the grammar sampler when it cannot constrain the candidates.
    pub fn constrain(&self, candidates: &mut LlamaTokenDataArray) -> Result<(), SamplerError> {
        match self {
            Self::Constrained(grammar_sampler) => grammar_sampler.apply(candidates),
            Self::Unconstrained => Ok(()),
        }
    }
}
