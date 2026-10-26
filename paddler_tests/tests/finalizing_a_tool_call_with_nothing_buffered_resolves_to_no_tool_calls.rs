#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::tool_call_pipeline_for_tools::tool_call_pipeline_for_tools;

#[test]
fn finalizing_a_tool_call_with_nothing_buffered_resolves_to_no_tool_calls() {
    let runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");

    assert_eq!(
        tool_call_pipeline_for_tools(&runtime.loaded_llama_model.model, &[])
            .finalize()
            .into_generated_token_result(),
        GeneratedTokenResult::ToolCallParsed(Vec::new())
    );
}
