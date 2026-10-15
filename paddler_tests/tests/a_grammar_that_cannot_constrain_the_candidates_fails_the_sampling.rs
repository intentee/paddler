#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;
use llama_cpp_bindings::error::SamplerApplyError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::ParseSpecialTokens;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn a_grammar_that_cannot_constrain_the_candidates_fails_the_sampling() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b_target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let mut llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let prompt_tokens = runtime
        .loaded_llama_model
        .model
        .str_to_token(
            "Paddler balances",
            AddBos::Always,
            ParseSpecialTokens::Always,
        )
        .expect("the prompt must tokenize");
    let mut batch = LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1);

    batch
        .add_sequence(&prompt_tokens, 0, false)
        .expect("the prompt must fit the batch");
    llama_context
        .decode(&mut batch)
        .expect("the prompt must decode");

    let smaller_vocabulary_runtime = LlamaRuntime::load(
        &nomic_embed_text_v1_5()
            .reference
            .cached_path(&Cache::from_env())
            .expect("the nomic embedding model must be in the Hugging Face cache"),
        runtime.inference_runtime_context.clone(),
    )
    .expect("the nomic embedding model must load");
    let grammar_of_a_smaller_vocabulary = LlamaSampler::grammar(
        &smaller_vocabulary_runtime.loaded_llama_model.model,
        r#"root ::= "a""#,
        "root",
    )
    .expect("the grammar must compile");

    assert_eq!(
        TokenSampling {
            chain: LlamaSampler::greedy().expect("the greedy sampler must initialize"),
            grammar: GrammarSampling::Constrained(grammar_of_a_smaller_vocabulary),
        }
        .sample(
            &llama_context,
            batch.n_tokens() - 1,
            &mut LlamaTokenDataArray::new(Vec::new(), false),
        )
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            TextGenerationError::GrammarConstraintFailed(SamplerApplyError::Reported {
                message: format!(
                    "vector::_M_range_check: __n (which is {n_vocab}) >= this->size() (which is {n_vocab})",
                    n_vocab = smaller_vocabulary_runtime.loaded_llama_model.model.n_vocab(),
                ),
            })
            .to_string()
        )
    );
}
