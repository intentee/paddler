#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_tests::grammar_refusing_the_greedy_selection::GrammarRefusingTheGreedySelection;
use paddler_tests::prompt_sampling_positions::PromptSamplingPositions;

#[test]
fn a_sampler_chain_that_refuses_its_own_selection_fails_the_sampling() {
    let refusal = GrammarRefusingTheGreedySelection::load()
        .expect("the grammar refusing the greedy selection must load");
    let mut llama_context = refusal
        .runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let PromptSamplingPositions { batch_position, .. } = refusal
        .decode_the_prompt(&mut llama_context)
        .expect("the prompt must decode");

    assert_eq!(
        TokenSampling {
            chain: LlamaSampler::chain([
                LlamaSampler::greedy().expect("the greedy sampler must initialize"),
                refusal.grammar().expect("the grammar must compile"),
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
