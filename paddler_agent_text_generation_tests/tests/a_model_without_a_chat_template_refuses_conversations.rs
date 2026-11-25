#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::conversation_generation::ConversationGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand as _;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_agent_text_generation_tests::model_capabilities_request_preparer::ModelCapabilitiesRequestPreparer;
use paddler_agent_text_generation_tests::qwen3_with_a_patched_chat_template::Qwen3WithAPatchedChatTemplate;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::conversation_feature::ConversationFeature;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::unsupported_conversation_feature::UnsupportedConversationFeature;
use paddler_model_card_fixture::served_context_size::served_context_size;

#[test]
fn a_model_without_a_chat_template_refuses_conversations() {
    let qwen3_without_a_chat_template = Qwen3WithAPatchedChatTemplate::without_a_chat_template()
        .expect("the model without a chat template must be written");
    let runtime = LlamaRuntime::load(
        &qwen3_without_a_chat_template.model_path(),
        agent_inference_runtime_context(1),
    )
    .expect("the model without a chat template must load");
    let request_preparer = ModelCapabilitiesRequestPreparer {
        llama_runtime: &runtime,
        model_capabilities_settings: default_model_capabilities_settings(),
        sequence_context_size: served_context_size().get(),
    }
    .into_generation_request_preparer()
    .expect("the capabilities of the model without a chat template must be established");
    let ConversationGeneration {
        agent_request,
        generate_tokens_stop_tx: _generate_tokens_stop_tx,
        mut generated_tokens_rx,
    } = ConversationGeneration::new(
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage::Message {
                content: ConversationMessageContent::Text("Name a primary color.".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: ModelDefaultOr::Explicit(false),
            grammar: None,
            max_tokens: NonZeroU32::MIN,
            parallel_tool_calls: ModelDefaultOr::ModelDefault,
            parse_tool_calls: false,
            tools: vec![],
        },
        runtime.inference_runtime_context.slot_aggregated_status,
    );

    let scheduler_command = request_preparer.prepare_scheduler_command(
        Some("test-agent-0"),
        TextGenerationRequest::ContinueFromConversationHistory(agent_request),
    );

    assert!(scheduler_command.is_none());
    assert_eq!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::ConversationFeatureNotSupported(
            UnsupportedConversationFeature {
                description:
                    "test-agent-0: the model has no chat template, so it cannot continue a conversation"
                        .to_owned(),
                feature: ConversationFeature::ChatTemplate,
            }
        ))
    );
}
