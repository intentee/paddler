use anyhow::Context;
use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::sampling::LlamaSampler;

use crate::agent::sampling_outcome::SamplingOutcome;

pub fn sample_token_at_batch_index(
    llama_context: &LlamaContext,
    batch_index: i32,
    chain: &mut LlamaSampler,
    grammar_sampler: &mut Option<LlamaSampler>,
) -> Result<SamplingOutcome> {
    let mut token_data_array = llama_context
        .token_data_array_ith(batch_index)
        .context("failed to read token data array for sampling")?;

    if let Some(grammar) = grammar_sampler.as_ref() {
        token_data_array.apply_sampler(grammar);
    }

    token_data_array.apply_sampler(chain);

    let Some(llama_token) = token_data_array.selected_token() else {
        return Ok(SamplingOutcome::AllCandidatesEliminated);
    };

    chain
        .accept(llama_token)
        .context("sampler chain failed to accept the selected token")?;

    if let Some(grammar) = grammar_sampler.as_mut()
        && let Err(err) = grammar.accept(llama_token)
    {
        return Ok(SamplingOutcome::GrammarRejectedModelOutput(err.to_string()));
    }

    Ok(SamplingOutcome::Token(llama_token))
}
