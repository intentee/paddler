#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::llama_batch::LlamaBatch;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

const SLOTS: u16 = 2;
const TOKENS_THE_BATCH_HOLDS: u32 = 1;

#[test]
fn a_warm_up_whose_batch_cannot_hold_every_slot_is_refused() {
    let runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the kev model must load");
    let mut llama_context = runtime
        .create_text_generation_llama_context(u32::from(SLOTS))
        .expect("the context must be created");
    let mut batch = LlamaBatch::new(ContextLimits {
        n_batch: TOKENS_THE_BATCH_HOLDS,
        ..ContextLimits::of(&llama_context)
    });

    assert_eq!(
        runtime
            .loaded_llama_model
            .warm_up_llama_context(&mut llama_context, &mut batch)
            .map_err(|warm_up_error| warm_up_error.to_string()),
        Err(
            AgentRuntimeError::WarmUpBatchAssemblyFailed(BatchAddError::BatchFull {
                n_batch: TOKENS_THE_BATCH_HOLDS,
            })
            .to_string()
        )
    );
}
