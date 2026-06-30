#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::sampling::LlamaSampler;
use paddler_agent::sample_token_at_batch_index::sample_token_at_batch_index;
use paddler_agent::sampling_outcome::SamplingOutcome;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn sample_token_at_batch_index_reports_all_candidates_eliminated_when_no_token_selected()
-> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let context = loaded.decoded_context()?;
    let mut chain = LlamaSampler::chain_simple([LlamaSampler::top_k(1)]);
    let mut grammar_sampler = None;

    let outcome = sample_token_at_batch_index(&context, 0, &mut chain, &mut grammar_sampler)?;

    assert!(matches!(outcome, SamplingOutcome::AllCandidatesEliminated));

    Ok(())
}
