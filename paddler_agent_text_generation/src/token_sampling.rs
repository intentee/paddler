use llama_cpp_context::llama_context::LlamaContext;
use llama_cpp_model::llama_token_data_array::LlamaTokenDataArray;
use llama_cpp_sampler::llama_sampler::LlamaSampler;
use llama_cpp_sampler::sampler_error::SamplerError;

use crate::grammar_sampling::GrammarSampling;
use crate::sampling_outcome::SamplingOutcome;
use crate::text_generation_error::TextGenerationError;

pub struct TokenSampling {
    pub chain: LlamaSampler,
    pub grammar: GrammarSampling,
}

impl TokenSampling {
    /// # Errors
    ///
    /// Returns [`TextGenerationError::TokenDataUnreadable`] when the logits of the batch position cannot be read, [`TextGenerationError::GrammarConstraintFailed`] when the grammar fails to constrain the candidates, [`TextGenerationError::SamplerChainApplicationFailed`] when the sampler chain fails to apply, and [`TextGenerationError::SamplerChainRejectedToken`] when the chain rejects the selected token.
    pub fn sample(
        &mut self,
        llama_context: &LlamaContext,
        batch_index: i32,
        candidates: &mut LlamaTokenDataArray,
    ) -> Result<SamplingOutcome, TextGenerationError> {
        llama_context
            .fill_token_data_array_ith(batch_index, candidates)
            .map_err(TextGenerationError::TokenDataUnreadable)?;
        match self.grammar.constrain(candidates) {
            Ok(()) => {}
            Err(SamplerError::EveryCandidateExcluded) => {
                return Ok(SamplingOutcome::GrammarRejectedModelOutput(
                    SamplerError::EveryCandidateExcluded.to_string(),
                ));
            }
            Err(constraint_error) => {
                return Err(TextGenerationError::GrammarConstraintFailed(
                    constraint_error,
                ));
            }
        }
        self.chain
            .apply(candidates)
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
