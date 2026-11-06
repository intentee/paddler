#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::error::chat_session_error::ChatSessionError;
use llama_cpp_bindings::model::llama_chat_template::LlamaChatTemplate;

use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::conversation_generation::ConversationGeneration;
use paddler_tests::first_byte_of_the_first_message_chat_template::first_byte_of_the_first_message_chat_template;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::chat_templates_request_preparer::ChatTemplatesRequestPreparer;

const MESSAGE_STARTING_WITH_A_TWO_BYTE_CHARACTER: &str = "é";

#[test]
fn a_conversation_whose_template_renders_invalid_utf8_cannot_be_tokenized() {
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
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage::Message {
                content: ConversationMessageContent::Text(
                    MESSAGE_STARTING_WITH_A_TWO_BYTE_CHARACTER.to_owned(),
                ),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: NonZeroU32::MIN,
            parse_tool_calls: false,
            tools: vec![],
        },
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );

    assert_eq!(
        ChatTemplatesRequestPreparer {
            chat_templates: ChatTemplates::with_override(
                model,
                &LlamaChatTemplate::new(&first_byte_of_the_first_message_chat_template().content)
                    .expect("the template holds no NUL byte"),
            )
            .expect("llama.cpp must prepare the template"),
            image_input: ImageInput::Unsupported,
            llama_runtime: &runtime,
            sequence_context_size: runtime.model_runtime_parameters.context_size.get(),
        }
        .into_generation_request_preparer()
        .prepare_conversation_history(agent_request)
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            TextGenerationError::ChatPromptTokenizationFailed(
                ChatSessionError::SessionTextNotUtf8(
                    String::from_utf8(
                        MESSAGE_STARTING_WITH_A_TWO_BYTE_CHARACTER.as_bytes()[..1].to_vec()
                    )
                    .expect_err("the first byte of a two byte character is not UTF-8")
                    .utf8_error()
                )
            )
            .to_string()
        )
    );
}
