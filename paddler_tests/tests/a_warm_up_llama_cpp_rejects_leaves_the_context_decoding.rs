#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_context::context_error::ContextError;
use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_context_memory::context_memory::ContextMemory as _;
use llama_cpp_model::llama_token::LlamaToken;
use llama_cpp_model::sampled_token::SampledToken;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_test_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

const SLOTS: u16 = 200;
const CACHE_CELLS_FEWER_THAN_TWO_PER_SLOT: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[test]
fn a_warm_up_llama_cpp_rejects_leaves_the_context_decoding() {
    let mut runtime = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the kev model must load");

    runtime.model_runtime_parameters.context_size = CACHE_CELLS_FEWER_THAN_TWO_PER_SLOT;

    let mut llama_context = runtime
        .create_decision_llama_context(u32::from(SLOTS))
        .expect("the context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());

    assert!(matches!(
        runtime
            .loaded_llama_model
            .warm_up_llama_context(&mut llama_context, &mut batch),
        Err(AgentRuntimeError::WarmUpDecodeFailed(
            ContextError::DecodeNoKvCacheSlot
        ))
    ));

    batch.clear();
    batch
        .add(SampledToken::Content(LlamaToken::new(0)), 0, &[0], true)
        .expect("a single token must fit the batch");
    llama_context
        .decode(&batch)
        .expect("the context must keep decoding after the warm-up was rejected");

    assert_eq!(
        llama_context
            .memory_seq_pos_max(0)
            .expect("the sequence position must be readable"),
        0
    );
}
