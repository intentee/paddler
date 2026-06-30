#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_agent::continuous_batch_arbiter::ContinuousBatchArbiter;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn continuous_batch_arbiter_warmup_decode_rejects_unallocatable_batch() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let model = loaded.model();
    let mut llama_context = loaded.new_context()?;

    let result =
        ContinuousBatchArbiter::run_warmup_decode(&model, &mut llama_context, usize::MAX, 1);

    assert!(
        result.is_err(),
        "warmup must fail hard when the requested batch size cannot be allocated"
    );

    Ok(())
}
