#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::oversized_text::oversized_text;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;

const SEQUENCE_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_conversation_exceeding_sequence_context() {
    let cluster = start_cluster_with_qwen3_and_context_size(
        vec![AgentConfig::single(1)],
        SEQUENCE_CONTEXT_SIZE,
    )
    .await
    .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![
                    ConversationMessage::Message {
                        content: ConversationMessageContent::Text(oversized_text()),
                        role: "user".to_owned(),
                    },
                ]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(20).unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert!(collected.token_results.iter().any(|result| matches!(
        &result.token_result,
        GeneratedTokenResult::PromptExceedsContextSize(details)
            if details.sequence_context_size == SEQUENCE_CONTEXT_SIZE.get()
                && details.prompt_tokens > SEQUENCE_CONTEXT_SIZE.get() as usize
    )));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
