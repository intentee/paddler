#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_chat::chat_error::ChatError;

use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::conversation_generation::ConversationGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_agent_text_generation_tests::first_byte_of_the_first_message_chat_template::first_byte_of_the_first_message_chat_template;
use paddler_agent_text_generation_tests::model_capabilities_request_preparer::ModelCapabilitiesRequestPreparer;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_model_card_fixture::served_context_size::served_context_size;

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
    let ConversationGeneration { agent_request, .. } = ConversationGeneration::new(
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage::Message {
                content: ConversationMessageContent::Text(
                    MESSAGE_STARTING_WITH_A_TWO_BYTE_CHARACTER.to_owned(),
                ),
                role: "user".to_owned(),
            }]),
            enable_thinking: ModelDefaultOr::Explicit(false),
            grammar: None,
            max_tokens: NonZeroU32::MIN,
            parallel_tool_calls: ModelDefaultOr::ModelDefault,
            parse_tool_calls: false,
            tools: vec![],
        },
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );

    assert!(matches!(
        ModelCapabilitiesRequestPreparer {
            llama_runtime: &runtime,
            model_capabilities_settings: ModelCapabilitiesSettings {
                chat_template_source: ChatTemplateSource::Override(
                    first_byte_of_the_first_message_chat_template()
                ),
                ..default_model_capabilities_settings()
            },
            sequence_context_size: served_context_size().get(),
        }
        .into_generation_request_preparer()
        .expect("llama.cpp must prepare the template")
        .prepare_conversation_history(agent_request),
        Err(TextGenerationError::ChatPromptTokenizationFailed(
            ChatError::SessionTextNotUtf8(_)
        ))
    ));
}
