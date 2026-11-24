#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_context::context_error::ContextError;
use llama_cpp_context::context_limits::ContextLimits;
use llama_cpp_context::llama_batch::LlamaBatch;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_test_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

const SLOTS: u16 = 2;
const TOKENS_THE_BATCH_HOLDS: u32 = 1;

#[test]
fn a_warm_up_whose_batch_cannot_hold_every_slot_is_refused() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the kev model must load");
    let mut llama_context = runtime
        .create_text_generation_llama_context(u32::from(SLOTS))
        .expect("the context must be created");
    let mut batch = LlamaBatch::new(ContextLimits {
        batch: TOKENS_THE_BATCH_HOLDS,
        ..llama_context.limits()
    });

    assert!(matches!(
        runtime
            .loaded_llama_model
            .warm_up_llama_context(&mut llama_context, &mut batch),
        Err(AgentRuntimeError::WarmUpBatchAssemblyFailed(
            ContextError::BatchFull {
                n_batch: TOKENS_THE_BATCH_HOLDS,
            }
        ))
    ));
}
