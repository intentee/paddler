#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::error::chat_session_error::ChatSessionError;
use llama_cpp_bindings_sys::LLAMA_RS_FAULT_POINT_CHAT_SESSION_GRAMMAR_NOT_UTF8;
use llama_cpp_fault_injection::armed_fault::ArmedFault;

use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::chat_templates_request_preparer::ChatTemplatesRequestPreparer;
use paddler_tests::conversation_generation::ConversationGeneration;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::weather_question_conversation::weather_question_conversation;

const BYTE_THE_FAULT_RETURNS_IN_PLACE_OF_THE_GRAMMAR: u8 = 0xff;

#[test]
fn a_conversation_whose_tool_call_grammar_llama_cpp_cannot_read_is_refused() {
    let runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let model = &runtime.loaded_llama_model.model;
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
    let request_preparer = ChatTemplatesRequestPreparer {
        chat_templates: ChatTemplates::new(model).expect("the model must embed a chat template"),
        image_input: ImageInput::Unsupported,
        llama_runtime: &runtime,
        sequence_context_size: runtime.model_runtime_parameters.context_size.get(),
    }
    .into_generation_request_preparer();
    let _grammar_cannot_be_read =
        ArmedFault::at(LLAMA_RS_FAULT_POINT_CHAT_SESSION_GRAMMAR_NOT_UTF8);

    assert_eq!(
        request_preparer
            .prepare_conversation_history(agent_request)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            TextGenerationError::ChatSessionGrammarUnavailable(
                ChatSessionError::SessionTextNotUtf8(
                    String::from_utf8(vec![BYTE_THE_FAULT_RETURNS_IN_PLACE_OF_THE_GRAMMAR])
                        .expect_err("a lone 0xff byte is not UTF-8")
                        .utf8_error()
                )
            )
            .to_string()
        )
    );
}
