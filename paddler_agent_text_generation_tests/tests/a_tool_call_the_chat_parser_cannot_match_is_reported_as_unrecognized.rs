#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_chat::chat_output::ChatOutput;
use llama_cpp_chat::chat_output_completeness::ChatOutputCompleteness;
use llama_cpp_model::sampled_token_section::SampledTokenSection;
use llama_cpp_token_usage::token_usage::TokenUsage;
use serde_json::Value;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::tool_call_event::ToolCallEvent;
use paddler_agent_text_generation_tests::get_weather_tool::get_weather_tool;
use paddler_agent_text_generation_tests::tool_call_pipeline_for_tools::tool_call_pipeline_for_tools;
use paddler_agent_text_generation_tests::weather_chat_session::weather_chat_session;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_model_card_fixture::ministral_3_3b_reasoning::ministral_3_3b_reasoning;

const TOOL_CALL_WITH_ARGUMENTS_THAT_ARE_NOT_JSON: &str = "[TOOL_CALLS]get_weather[ARGS]not json";

#[test]
fn a_tool_call_the_chat_parser_cannot_match_is_reported_as_unrecognized() {
    let runtime = LlamaRuntime::load(
        &ministral_3_3b_reasoning()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let model = &runtime.loaded_llama_model.model;
    let reading = weather_chat_session(
        model,
        model
            .chat_templates()
            .expect("the model must embed a chat template"),
    )
    .expect("the model's template must serve the weather question")
    .read(
        &ChatOutput::from(TOOL_CALL_WITH_ARGUMENTS_THAT_ARE_NOT_JSON),
        ChatOutputCompleteness::Finished,
    )
    .expect("the answer must be read");

    assert_eq!(
        tool_call_pipeline_for_tools(&[Tool::<Value>::from(get_weather_tool())])
            .expect("the tools must validate")
            .resolve_at_completion(
                &reading,
                SampledTokenSection::ToolCall,
                &TokenUsage {
                    tool_call: 1,
                    ..TokenUsage::default()
                },
            )
            .map(ToolCallEvent::into_generated_token_result),
        Some(GeneratedTokenResult::UnrecognizedToolCallFormat(
            RawToolCallTokens {
                text: TOOL_CALL_WITH_ARGUMENTS_THAT_ARE_NOT_JSON.to_owned(),
                expected_format: "peg-native".to_owned(),
            }
        ))
    );
}
