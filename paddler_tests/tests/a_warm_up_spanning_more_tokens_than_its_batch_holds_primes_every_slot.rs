#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::llama_batch::LlamaBatch;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

const SLOTS: u16 = 2;

#[test]
fn a_warm_up_spanning_more_tokens_than_its_batch_holds_primes_every_slot() {
    let mut runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the kev model must load");

    runtime.model_runtime_parameters.n_batch =
        BatchSize::try_from(u32::from(SLOTS)).expect("a batch of one token per slot is valid");

    let mut llama_context = runtime
        .create_text_generation_llama_context(u32::from(SLOTS))
        .expect("the context must be created");
    let mut batch = LlamaBatch::new(
        runtime.model_runtime_parameters.n_batch.capacity(),
        i32::from(SLOTS),
    );

    assert_eq!(
        runtime
            .loaded_llama_model
            .warm_up_llama_context(&mut llama_context, &mut batch, SLOTS)
            .as_ref()
            .map_err(ToString::to_string),
        Ok(&())
    );
}
