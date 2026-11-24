#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_context_memory::context_memory::ContextMemory as _;
use llama_cpp_model::llama_token::LlamaToken;
use llama_cpp_model::sampled_token::SampledToken;

use paddler_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

const SLOTS: u16 = 2;

#[test]
fn a_warm_up_spanning_more_tokens_than_its_batch_holds_primes_every_slot() {
    let mut runtime = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the kev model must load");

    runtime.model_runtime_parameters.n_batch =
        BatchSize::try_from(u32::from(SLOTS)).expect("a batch of one token per slot is valid");

    let mut llama_context = runtime
        .create_text_generation_llama_context(u32::from(SLOTS))
        .expect("the context must be created");
    let mut batch = LlamaBatch::new(llama_context.limits());

    runtime
        .loaded_llama_model
        .warm_up_llama_context(&mut llama_context, &mut batch)
        .expect("the warm-up must prime every slot");

    batch.clear();

    for sequence_id in 0..i32::from(SLOTS) {
        batch
            .add(
                SampledToken::Content(LlamaToken::new(0)),
                0,
                &[sequence_id],
                true,
            )
            .expect("a token for every slot must fit the batch");
    }

    llama_context
        .decode(&batch)
        .expect("every warmed up slot must decode");

    assert_eq!(
        (0..i32::from(SLOTS))
            .map(|sequence_id| {
                llama_context
                    .memory_seq_pos_max(sequence_id)
                    .expect("the sequence position must be readable")
            })
            .collect::<Vec<i32>>(),
        vec![0; usize::from(SLOTS)]
    );
}
