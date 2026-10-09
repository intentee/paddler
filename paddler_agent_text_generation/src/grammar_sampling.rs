use llama_cpp_bindings::error::SamplerAcceptError;
use llama_cpp_bindings::error::SamplerApplyError;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::LlamaToken;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

pub enum GrammarSampling {
    Constrained(LlamaSampler),
    Unconstrained,
}

impl GrammarSampling {
    pub fn constrain(&self, candidates: &mut LlamaTokenDataArray) -> Result<(), SamplerApplyError> {
        match self {
            Self::Constrained(grammar_sampler) => candidates.apply_sampler(grammar_sampler),
            Self::Unconstrained => Ok(()),
        }
    }

    pub fn accept(&mut self, token: LlamaToken) -> Result<(), SamplerAcceptError> {
        match self {
            Self::Constrained(grammar_sampler) => grammar_sampler.accept(token),
            Self::Unconstrained => Ok(()),
        }
    }
}
