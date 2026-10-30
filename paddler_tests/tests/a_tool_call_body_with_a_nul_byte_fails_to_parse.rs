#![cfg(feature = "tests_that_use_llms")]

use std::ffi::CString;

use llama_cpp_bindings::error::ParseChatMessageError;
use llama_cpp_bindings::token_piece::TokenPiece;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::tool_call_pipeline_for_tools::tool_call_pipeline_for_tools;

const TOOL_CALL_BODY_WITH_A_NUL_BYTE: &str = "hello\0world";

#[test]
fn a_tool_call_body_with_a_nul_byte_fails_to_parse() {
    let runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let mut tool_call_pipeline =
        tool_call_pipeline_for_tools(&runtime.loaded_llama_model.model, &[]);

    tool_call_pipeline.feed(&TokenPiece::Visible(
        TOOL_CALL_BODY_WITH_A_NUL_BYTE.to_owned(),
    ));

    assert_eq!(
        tool_call_pipeline.finalize().into_generated_token_result(),
        GeneratedTokenResult::ToolCallParseFailed(
            ParseChatMessageError::InputContainsNulByte(
                CString::new(TOOL_CALL_BODY_WITH_A_NUL_BYTE)
                    .expect_err("a body with a NUL byte must not become a C string")
            )
            .to_string()
        )
    );
}
