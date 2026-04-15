use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::LlamaToken;
use paddler_types::generated_token_result::GeneratedTokenResult;

pub fn sample_token_at_batch_index(
    llama_context: &LlamaContext,
    batch_index: i32,
    chain: &mut LlamaSampler,
    grammar_sampler: &mut Option<LlamaSampler>,
) -> Result<LlamaToken, GeneratedTokenResult> {
    let mut token_data_array = llama_context
        .token_data_array_ith(batch_index)
        .map_err(|err| GeneratedTokenResult::SamplerError(err.to_string()))?;

    if let Some(grammar) = grammar_sampler.as_ref() {
        token_data_array.apply_sampler(grammar);
    }

    token_data_array.apply_sampler(chain);

    let token = token_data_array.selected_token().ok_or_else(|| {
        GeneratedTokenResult::SamplerError(
            "all token candidates were eliminated during sampling".to_owned(),
        )
    })?;

    chain
        .accept(token)
        .map_err(|err| GeneratedTokenResult::SamplerError(err.to_string()))?;

    if let Some(grammar) = grammar_sampler.as_mut() {
        grammar
            .accept(token)
            .map_err(|err| GeneratedTokenResult::GrammarRejectedModelOutput(err.to_string()))?;
    }

    Ok(token)
}
