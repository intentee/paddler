#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::chat_output::ChatOutput;
use llama_cpp_bindings::chat_output_completeness::ChatOutputCompleteness;
use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::error::parse_chat_message_error::ParseChatMessageError;
use llama_cpp_bindings::sampled_token_section::SampledTokenSection;
use llama_cpp_bindings_sys::LLAMA_RS_FAULT_POINT_CHAT_READING_PARSE_THROWS;
use llama_cpp_bindings_types::token_usage::TokenUsage;
use llama_cpp_fault_injection::armed_fault::ArmedFault;
use llama_cpp_fault_injection::injected_fault_message::injected_fault_message;
use serde_json::Value;

use paddler_agent_text_generation::tool_call_event::ToolCallEvent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_test_cluster_harness::model_card::ministral_3_3b_reasoning::ministral_3_3b_reasoning;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::weather_chat_session::weather_chat_session;

use crate::tool_call_pipeline_for_tools::tool_call_pipeline_for_tools;

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
        &ChatTemplates::new(model).expect("the model must embed a chat template"),
    )
    .expect("the model's template must serve the weather question")
    .read(
        &ChatOutput::from(WEATHER_TOOL_CALL),
        ChatOutputCompleteness::Finished,
    )
    .expect("the answer must be read");
    let _mapping_fails = ArmedFault::at(LLAMA_RS_FAULT_POINT_CHAT_READING_PARSE_THROWS);

    assert_eq!(
        tool_call_pipeline_for_tools(&[Tool::<Value>::from(get_weather_tool())])
            .resolve_at_completion(
                &reading,
                SampledTokenSection::ToolCall,
                &TokenUsage::default(),
            )
            .map(ToolCallEvent::into_generated_token_result),
        Some(GeneratedTokenResult::ToolCallParseFailed(
            ParseChatMessageError::Reported {
                message: injected_fault_message(),
            }
            .to_string()
        ))
    );
}
