#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_agent::sample_token_at_batch_index::sample_token_at_batch_index;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn sample_token_at_batch_index_errors_on_null_chain_sampler() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let context = loaded.decoded_context()?;
    let mut null_chain = LlamaSampler {
        sampler: std::ptr::null_mut(),
    };
    let mut grammar_sampler = None;

    let outcome = sample_token_at_batch_index(&context, 0, &mut null_chain, &mut grammar_sampler);

    assert!(outcome.is_err());

    Ok(())
}
