#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::token_piece::TokenPiece;
use serde_json::Value;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_test_cluster_harness::model_card::ministral_3_3b_reasoning::ministral_3_3b_reasoning;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::tool_call_pipeline_for_tools::tool_call_pipeline_for_tools;

const TOOL_CALL_OPEN_MARKER: &str = "[TOOL_CALLS]";
const TOOL_CALL_WITH_ARGUMENTS_THAT_ARE_NOT_JSON: &str = "get_weather[ARGS]not json";

#[test]
fn a_tool_call_the_chat_parser_cannot_match_is_reported_as_unrecognized() {
    let runtime = LlamaRuntime::load(
        &ministral_3_3b_reasoning()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let mut tool_call_pipeline = tool_call_pipeline_for_tools(
        &runtime.loaded_llama_model.model,
        &[Tool::<Value>::from(get_weather_tool())],
    );

    tool_call_pipeline.feed(&TokenPiece::Marker(TOOL_CALL_OPEN_MARKER.to_owned()));
    tool_call_pipeline.feed(&TokenPiece::Visible(
        TOOL_CALL_WITH_ARGUMENTS_THAT_ARE_NOT_JSON.to_owned(),
    ));

    assert_eq!(
        tool_call_pipeline.finalize().into_generated_token_result(),
        GeneratedTokenResult::UnrecognizedToolCallFormat(RawToolCallTokens {
            text: format!("{TOOL_CALL_OPEN_MARKER}{TOOL_CALL_WITH_ARGUMENTS_THAT_ARE_NOT_JSON}"),
            ffi_error_message:
                "The model produced output that does not match the expected peg-native format"
                    .to_owned(),
        })
    );
}
