#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_sampler::llama_sampler::LlamaSampler;
use llama_cpp_sampler::sampler_error::SamplerError;

use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_agent_text_generation_tests::sampling_with_a_smaller_vocabulary_grammar::SamplingWithASmallerVocabularyGrammar;
use paddler_model_card_fixture::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

#[test]
fn a_sampler_chain_that_cannot_rank_the_candidates_fails_the_sampling() {
    let sampling = SamplingWithASmallerVocabularyGrammar::load(
        &nomic_embed_text_v1_5()
            .cached_path()
            .expect("the nomic embedding model must be in the Hugging Face cache"),
    )
    .expect("the models must load");

    assert!(matches!(
        sampling
            .sample_the_prompt_with(|grammar_of_a_smaller_vocabulary| TokenSampling {
                chain: LlamaSampler::chain([grammar_of_a_smaller_vocabulary])
                    .expect("the sampler chain must initialize"),
                grammar: GrammarSampling::Unconstrained,
            })
            .expect("the prompt must be ready to sample"),
        Err(TextGenerationError::SamplerChainApplicationFailed(
            SamplerError::ApplyReported { .. }
        ))
    ));
}
