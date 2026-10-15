#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::sync::mpsc;

use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand as _;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::text_only_generation_parts::TextOnlyGenerationParts;
use paddler_tests::tools_serializer_that_fails::ToolsSerializerThatFails;

#[test]
fn tools_that_cannot_be_serialized_refuse_the_generation() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b_target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let preparer = runtime.text_only_generation_request_preparer(
        TextOnlyGenerationParts {
            chat_template_renderer:
                ChatTemplateRenderer::new(first_message_content_chat_template())
                    .expect("the chat template must compile"),
            sequence_context_size: runtime.model_runtime_parameters.context_size.get(),
            streaming_markers: runtime
                .loaded_llama_model
                .model
                .streaming_markers()
                .expect("the streaming markers must be detected"),
        },
        runtime.sampler_chain_factory(),
        ToolsSerializerThatFails,
    );
    let (generated_tokens_tx, mut generated_tokens_rx) = mpsc::unbounded_channel();
    let (_generate_tokens_stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel();

    let scheduler_command = preparer.prepare_scheduler_command(
        Some("test-agent"),
        TextGenerationRequest::ContinueFromConversationHistory(AgentRequest {
            params: ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Hello".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: true,
                tools: vec![get_weather_tool()],
            },
            response_tx: generated_tokens_tx,
            slot_guard: SlotGuard::new(runtime.inference_runtime_context.slot_aggregated_status),
            stop_rx: generate_tokens_stop_rx,
        }),
    );

    assert!(scheduler_command.is_none());
    assert_eq!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::ToolsSerializationFailed(
            "test-agent: failed to serialize tools: tools cannot be serialized".to_owned()
        ))
    );
}
