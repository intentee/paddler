#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::conversation_generation::ConversationGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

#[test]
fn a_conversation_asking_a_template_that_cannot_think_to_think_is_refused() {
    let runtime = LlamaRuntime::load(
        &smolvlm2_256m()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let sequence_context_size = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created")
        .n_ctx_seq();
    let request_preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size,
    }
    .into_generation_request_preparer()
    .expect("the first message content chat template must be prepared");
    let ConversationGeneration { agent_request, .. } = ConversationGeneration::new(
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage::Message {
                content: ConversationMessageContent::Text("Paddler".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: ModelDefaultOr::Explicit(true),
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
        request_preparer.prepare_conversation_history(agent_request),
        Err(TextGenerationError::ThinkingUnsupported)
    ));
}
