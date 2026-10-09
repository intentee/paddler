use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use crate::grammar_sampling::GrammarSampling;
use crate::sampling_outcome::SamplingOutcome;
use crate::text_generation_error::TextGenerationError;

pub struct TokenSampling {
    pub chain: LlamaSampler,
    pub grammar: GrammarSampling,
}

impl TokenSampling {
    pub fn sample(
        &mut self,
        llama_context: &LlamaContext,
        batch_index: i32,
        candidates: &mut LlamaTokenDataArray,
    ) -> Result<SamplingOutcome, TextGenerationError> {
        llama_context
            .fill_token_data_array_ith(batch_index, candidates)
            .map_err(TextGenerationError::TokenDataUnreadable)?;
        self.grammar
            .constrain(candidates)
            .map_err(TextGenerationError::GrammarConstraintFailed)?;
        candidates
            .apply_sampler(&self.chain)
            .map_err(TextGenerationError::SamplerChainApplicationFailed)?;

        let Some(token) = candidates.selected_token() else {
            return Ok(SamplingOutcome::AllCandidatesEliminated);
        };

        self.chain
            .accept(token)
            .map_err(TextGenerationError::SamplerChainRejectedToken)?;

        if let Err(grammar_rejection) = self.grammar.accept(token) {
            return Ok(SamplingOutcome::GrammarRejectedModelOutput(
                grammar_rejection.to_string(),
            ));
        }

        Ok(SamplingOutcome::Token(token))
    }
}
