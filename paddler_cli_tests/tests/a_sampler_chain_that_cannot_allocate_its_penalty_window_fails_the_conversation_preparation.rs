#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_cli_tests::generation_preparation_without_memory_for_the_penalty_window::GenerationPreparationWithoutMemoryForThePenaltyWindow;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::conversation_generation::ConversationGeneration;

#[test]
fn a_sampler_chain_that_cannot_allocate_its_penalty_window_fails_the_conversation_preparation() {
    let preparation = GenerationPreparationWithoutMemoryForThePenaltyWindow::load()
        .expect("the generation preparation must load");
    let ConversationGeneration { agent_request, .. } = ConversationGeneration::new(
        ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage::Message {
                content: ConversationMessageContent::Text("Paddler".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: NonZeroU32::MIN,
            parse_tool_calls: false,
            tools: vec![],
        },
        preparation
            .llama_runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );

    GenerationPreparationWithoutMemoryForThePenaltyWindow::refuse_the_penalty_window_allocation()
        .expect("the penalty window allocation must be refused");

    assert_eq!(
        preparation
            .request_preparer
            .prepare_conversation_history(agent_request)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(GenerationPreparationWithoutMemoryForThePenaltyWindow::penalty_window_allocation_failure())
    );
}
