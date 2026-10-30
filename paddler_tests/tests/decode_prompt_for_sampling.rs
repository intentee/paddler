#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::ParseSpecialTokens;

use paddler_tests::llama_runtime::LlamaRuntime;

pub fn decode_prompt_for_sampling(
    runtime: &LlamaRuntime,
    llama_context: &mut LlamaContext<'_>,
    prompt: &str,
) -> i32 {
    let prompt_tokens = runtime
        .loaded_llama_model
        .model
        .str_to_token(prompt, AddBos::Always, ParseSpecialTokens::Always)
        .expect("the prompt must tokenize");
    let mut batch = LlamaBatch::new(runtime.model_runtime_parameters.n_batch.capacity(), 1);

    batch
        .add_sequence(&prompt_tokens, 0, false)
        .expect("the prompt must fit the batch");
    llama_context
        .decode(&mut batch)
        .expect("the prompt must decode");

    batch.n_tokens() - 1
}
