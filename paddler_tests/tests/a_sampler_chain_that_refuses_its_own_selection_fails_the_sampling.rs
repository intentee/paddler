#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::token_sampling::TokenSampling;

use crate::grammar_refusing_the_greedy_selection::GrammarRefusingTheGreedySelection;

#[test]
fn a_sampler_chain_that_refuses_its_own_selection_fails_the_sampling() {
    let refusal = GrammarRefusingTheGreedySelection::load();
    let mut llama_context = refusal
        .runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let batch_position = refusal.decode_the_prompt(&mut llama_context);

    assert_eq!(
        TokenSampling {
            chain: LlamaSampler::chain_simple([
                LlamaSampler::greedy().expect("the greedy sampler must initialize"),
                refusal.grammar(),
            ])
            .expect("the sampler chain must initialize"),
            grammar: GrammarSampling::Unconstrained,
        }
        .sample(
            &llama_context,
            batch_position,
            &mut LlamaTokenDataArray::new(Vec::new(), false),
        )
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            TextGenerationError::SamplerChainRejectedToken(
                GrammarRefusingTheGreedySelection::refusal()
            )
            .to_string()
        )
    );
}
