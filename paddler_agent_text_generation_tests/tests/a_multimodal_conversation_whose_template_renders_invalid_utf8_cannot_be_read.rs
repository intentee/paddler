#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_chat::chat_error::ChatError;

use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::conversation_generation::ConversationGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_agent_text_generation_tests::first_byte_of_the_first_message_chat_template::first_byte_of_the_first_message_chat_template;
use paddler_agent_text_generation_tests::model_capabilities_request_preparer::ModelCapabilitiesRequestPreparer;
use paddler_file_fixture::image_fixture::ImageFixture;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_card_fixture::served_context_size::served_context_size;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;
use paddler_model_card_fixture::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

const QUESTION_STARTING_WITH_A_TWO_BYTE_CHARACTER: &str = "Économisez: how many llamas?";

#[test]
fn a_multimodal_conversation_whose_template_renders_invalid_utf8_cannot_be_read() {
    let runtime = LlamaRuntime::load(
        &smolvlm2_256m()
            .cached_path()
            .expect("the vision model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the vision model must load");
    let ConversationGeneration { agent_request, .. } = ConversationGeneration::new(
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage::Message {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::Text {
                        text: QUESTION_STARTING_WITH_A_TWO_BYTE_CHARACTER.to_owned(),
                    },
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: ImageFixture::LlamasJpeg
                                .data_uri()
                                .expect("the test image must load"),
                        },
                    },
                ]),
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
                multimodal_projection: MultimodalProjection::File(
                    smolvlm2_256m_mmproj()
                        .cached_path()
                        .expect("the multimodal projection must be in the Hugging Face cache"),
                ),
                ..default_model_capabilities_settings()
            },
            sequence_context_size: served_context_size().get(),
        }
        .into_generation_request_preparer()
        .expect("llama.cpp must prepare the template and the multimodal projection")
        .prepare_conversation_history(agent_request),
        Err(TextGenerationError::ChatPromptUnreadable(
            ChatError::SessionTextNotUtf8(_)
        ))
    ));
}
