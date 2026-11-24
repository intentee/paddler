#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings_sys::bindings::LLAMA_RS_FAULT_POINT_CHAT_READING_PARSE_THROWS;
use llama_cpp_chat::chat_output::ChatOutput;
use llama_cpp_chat::chat_output_completeness::ChatOutputCompleteness;
use llama_cpp_fault_injection::armed_fault::ArmedFault;
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
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_model_card_fixture::ministral_3_3b_reasoning::ministral_3_3b_reasoning;

const WEATHER_TOOL_CALL: &str = r#"[TOOL_CALLS]get_weather[ARGS]{"location": "Paris"}"#;

#[test]
fn a_tool_call_answer_llama_cpp_fails_to_map_is_reported_as_a_parse_failure() {
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
        &ChatOutput::from(WEATHER_TOOL_CALL),
        ChatOutputCompleteness::Finished,
    )
    .expect("the answer must be read");
    let _mapping_fails = ArmedFault::at(LLAMA_RS_FAULT_POINT_CHAT_READING_PARSE_THROWS);

    assert!(matches!(
        tool_call_pipeline_for_tools(&[Tool::<Value>::from(get_weather_tool())])
            .expect("the tools must validate")
            .resolve_at_completion(
                &reading,
                SampledTokenSection::ToolCall,
                &TokenUsage::default(),
            )
            .map(ToolCallEvent::into_generated_token_result),
        Some(GeneratedTokenResult::ToolCallParseFailed(_))
    ));
}
