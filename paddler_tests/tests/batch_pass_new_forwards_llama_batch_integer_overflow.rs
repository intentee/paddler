#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::batch_add_error::BatchAddError;
use paddler_agent::continuous_batch_scheduler::batch_pass::BatchPass;

#[test]
fn batch_pass_new_forwards_llama_batch_integer_overflow() -> Result<()> {
    let Err(error) = BatchPass::new(usize::MAX, 1) else {
        return Err(anyhow!("an oversized n_batch must fail batch allocation"));
    };

    let Some(BatchAddError::IntegerOverflow(_)) = error.downcast_ref::<BatchAddError>() else {
        return Err(anyhow!("the failure must be a llama.cpp IntegerOverflow"));
    };

    Ok(())
}
