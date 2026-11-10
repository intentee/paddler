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
use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
use paddler_messaging::image_url::ImageUrl;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::chat_templates_request_preparer::ChatTemplatesRequestPreparer;
use paddler_tests::conversation_generation::ConversationGeneration;
use paddler_tests::first_byte_of_the_first_message_chat_template::first_byte_of_the_first_message_chat_template;
use paddler_tests::llama_runtime::LlamaRuntime;

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
    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
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
                            url: load_test_image_data_uri().expect("the test image must load"),
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

    assert_eq!(
        ChatTemplatesRequestPreparer {
            chat_templates: ChatTemplates::with_override(
                model,
                &LlamaChatTemplate::new(&first_byte_of_the_first_message_chat_template().content)
                    .expect("the template holds no NUL byte"),
            )
            .expect("llama.cpp must prepare the template"),
            image_input: ImageInput::Supported(
                runtime
                    .multimodal_prompt_support(
                        &llama_context,
                        &smolvlm2_256m_mmproj()
                            .cached_path()
                            .expect("the multimodal projection must be in the Hugging Face cache"),
                    )
                    .expect("the multimodal projection must load"),
            ),
            llama_runtime: &runtime,
            sequence_context_size,
        }
        .into_generation_request_preparer()
        .prepare_conversation_history(agent_request)
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            TextGenerationError::ChatPromptUnreadable(ChatSessionError::SessionTextNotUtf8(
                String::from_utf8(
                    QUESTION_STARTING_WITH_A_TWO_BYTE_CHARACTER
                        .as_bytes()
                        .first_chunk::<1>()
                        .expect("the question must start with a byte")
                        .to_vec()
                )
                .expect_err("the first byte of a two byte character is not UTF-8")
                .utf8_error()
            ))
            .to_string()
        )
    );
}
