#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_agent::sample_token_at_batch_index::sample_token_at_batch_index;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn sample_token_at_batch_index_errors_on_uninitialized_logits() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let context = loaded.new_context()?;
    let mut chain = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
    let mut grammar_sampler = None;

    let outcome = sample_token_at_batch_index(&context, 0, &mut chain, &mut grammar_sampler);

    assert!(outcome.is_err());

    Ok(())
}
