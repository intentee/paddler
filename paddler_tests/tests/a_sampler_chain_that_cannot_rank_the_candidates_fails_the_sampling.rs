#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::SamplerApplyError;
use llama_cpp_bindings::sampling::LlamaSampler;

use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

use crate::sampling_with_a_smaller_vocabulary_grammar::SamplingWithASmallerVocabularyGrammar;

#[test]
fn a_sampler_chain_that_cannot_rank_the_candidates_fails_the_sampling() {
    let sampling = SamplingWithASmallerVocabularyGrammar::load(
        &nomic_embed_text_v1_5()
            .cached_path()
            .expect("the nomic embedding model must be in the Hugging Face cache"),
    );

    assert_eq!(
        sampling
            .sample_the_prompt_with(|grammar_of_a_smaller_vocabulary| TokenSampling {
                chain: LlamaSampler::chain_simple([grammar_of_a_smaller_vocabulary])
                    .expect("the sampler chain must initialize"),
                grammar: GrammarSampling::Unconstrained,
            })
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TextGenerationError::SamplerChainApplicationFailed(SamplerApplyError::Reported {
                message: format!(
                    "vector::_M_range_check: __n (which is {n_vocab}) >= this->size() (which is {n_vocab})",
                    n_vocab = sampling.smaller_vocabulary_model.n_vocab(),
                ),
            })
            .to_string()
        )
    );
}
