#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::token::LlamaToken;
use paddler_agent::continuous_batch_scheduler::classified_token::ClassifiedToken;
use paddler_agent::continuous_batch_scheduler::tool_call_pass;
use paddler_agent::tool_call_pipeline::ToolCallPipeline;
use paddler_agent::tool_call_validator::ToolCallValidator;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::loaded_test_model::LoadedTestModel;

#[test]
fn tool_call_pass_finalizes_buffer_on_tool_call_section_exit() -> Result<()> {
    let loaded = LoadedTestModel::qwen3()?;
    let validator = ToolCallValidator::from_tools(&[])?;
    let mut pipeline = ToolCallPipeline::new(loaded.model(), &[], validator);

    let entering_tool_call = ClassifiedToken {
        sampled_token: SampledToken::ToolCall(LlamaToken::new(1)),
        was_in_tool_call: false,
        is_in_tool_call: true,
        visible_piece: String::new(),
        raw_piece: "this is not a tool call".to_owned(),
    };

    assert!(tool_call_pass::run(Some(&mut pipeline), &entering_tool_call).is_none());

    let exiting_tool_call = ClassifiedToken {
        sampled_token: SampledToken::Content(LlamaToken::new(2)),
        was_in_tool_call: true,
        is_in_tool_call: false,
        visible_piece: String::new(),
        raw_piece: String::new(),
    };

    let finalized = tool_call_pass::run(Some(&mut pipeline), &exiting_tool_call);

    assert!(matches!(
        finalized,
        Some(GeneratedTokenResult::ToolCallParsed(_))
    ));

    Ok(())
}
