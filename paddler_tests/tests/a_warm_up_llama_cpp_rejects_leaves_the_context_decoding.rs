#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;

use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;

const SEQUENCES_THE_CONTEXT_HOLDS: u32 = 1;
const SLOTS_THE_WARM_UP_PRIMES: u16 = 2;

#[test]
fn a_warm_up_llama_cpp_rejects_leaves_the_context_decoding() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b_target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let mut llama_context = runtime
        .create_text_generation_llama_context(SEQUENCES_THE_CONTEXT_HOLDS)
        .expect("the context must be created");
    let mut batch = LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1);

    runtime.loaded_llama_model.warm_up_llama_context(
        &mut llama_context,
        &mut batch,
        SLOTS_THE_WARM_UP_PRIMES,
    );

    batch.clear();
    batch
        .add(&SampledToken::Content(LlamaToken::new(0)), 0, &[0], true)
        .expect("a single token must fit the batch");
    llama_context
        .decode(&mut batch)
        .expect("the context must keep decoding after the warm-up was rejected");

    assert_eq!(
        llama_context
            .kv_cache_seq_pos_max(0)
            .expect("the sequence position must be readable"),
        0
    );
}
