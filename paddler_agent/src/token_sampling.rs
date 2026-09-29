use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use crate::grammar_sampling::GrammarSampling;
use crate::sampling_outcome::SamplingOutcome;

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
    ) -> Result<SamplingOutcome> {
        llama_context
            .fill_token_data_array_ith(batch_index, candidates)
            .context("failed to read token data array for sampling")?;
        self.grammar
            .constrain(candidates)
            .context("failed to apply grammar sampler to token data array")?;
        candidates
            .apply_sampler(&self.chain)
            .context("failed to apply sampler chain to token data array")?;

        let Some(token) = candidates.selected_token() else {
            return Ok(SamplingOutcome::AllCandidatesEliminated);
        };

        self.chain
            .accept(token)
            .context("sampler chain failed to accept the selected token")?;

        if let Err(grammar_rejection) = self.grammar.accept(token) {
            return Ok(SamplingOutcome::GrammarRejectedModelOutput(
                grammar_rejection.to_string(),
            ));
        }

        Ok(SamplingOutcome::Token(token))
    }
}
