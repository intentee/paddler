#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings_sys::bindings::LLAMA_RS_FAULT_POINT_CHAT_SESSION_GRAMMAR_NOT_UTF8;
use llama_cpp_chat::chat_error::ChatError;
use llama_cpp_fault_injection::armed_fault::ArmedFault;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::conversation_generation::ConversationGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_agent_text_generation_tests::gbnf_literal::gbnf_literal;
use paddler_agent_text_generation_tests::get_weather_tool::get_weather_tool;
use paddler_agent_text_generation_tests::model_capabilities_request_preparer::ModelCapabilitiesRequestPreparer;
use paddler_agent_text_generation_tests::weather_question_conversation::weather_question_conversation;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_model_card_fixture::served_context_size::served_context_size;

#[test]
fn a_conversation_whose_tool_call_grammar_llama_cpp_cannot_read_is_refused() {
    let runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let ConversationGeneration { agent_request, .. } = ConversationGeneration::new(
        ContinueFromConversationHistoryParams {
            grammar: None,
            ..weather_question_conversation(
                gbnf_literal("unused"),
                NonZeroU32::MIN,
                vec![get_weather_tool()],
            )
        },
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );
    let request_preparer = ModelCapabilitiesRequestPreparer {
        llama_runtime: &runtime,
        model_capabilities_settings: default_model_capabilities_settings(),
        sequence_context_size: served_context_size().get(),
    }
    .into_generation_request_preparer()
    .expect("the capabilities of the model must be established");
    let _grammar_cannot_be_read =
        ArmedFault::at(LLAMA_RS_FAULT_POINT_CHAT_SESSION_GRAMMAR_NOT_UTF8);

    assert!(matches!(
        request_preparer.prepare_conversation_history(agent_request),
        Err(TextGenerationError::ChatSessionGrammarUnavailable(
            ChatError::SessionTextNotUtf8(_)
        ))
    ));
}
