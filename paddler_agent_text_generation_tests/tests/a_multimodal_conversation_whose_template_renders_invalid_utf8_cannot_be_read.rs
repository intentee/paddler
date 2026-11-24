#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_chat::chat_error::ChatError;
use llama_cpp_model::llama_chat_template::LlamaChatTemplate;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::conversation_generation::ConversationGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::conversation_chat_templates::ConversationChatTemplates;
use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::chat_templates_request_preparer::ChatTemplatesRequestPreparer;
use paddler_agent_text_generation_tests::first_byte_of_the_first_message_chat_template::first_byte_of_the_first_message_chat_template;
use paddler_file_fixture::constants::LLAMAS_JPEG;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

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
    let model = &runtime.loaded_llama_model.model;
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
                            url: LLAMAS_JPEG.data_uri().expect("the test image must load"),
                        },
                    },
                ]),
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

    assert!(matches!(
        ChatTemplatesRequestPreparer {
            chat_templates: ConversationChatTemplates::Override(
                model
                    .chat_templates_with_override(
                        &LlamaChatTemplate::new(
                            &first_byte_of_the_first_message_chat_template().content
                        )
                        .expect("the template holds no NUL byte"),
                    )
                    .expect("llama.cpp must prepare the template"),
            ),
            image_input: ImageInput::Supported,
            llama_runtime: &runtime,
            sequence_context_size: runtime.model_runtime_parameters.context_size.get(),
        }
        .into_generation_request_preparer()
        .prepare_conversation_history(agent_request),
        Err(TextGenerationError::ChatPromptUnreadable(
            ChatError::SessionTextNotUtf8(_)
        ))
    ));
}
