#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;
use tokio_util::sync::CancellationToken;

const SEQUENCE_CONTEXT_SIZE: u32 = 256;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_conversation_exceeding_sequence_context() -> Result<()> {
    let cluster = start_cluster_with_qwen3_and_context_size(
        vec![AgentConfig::single(1)],
        SEQUENCE_CONTEXT_SIZE,
    )
    .await?;

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "The quick brown fox jumps over the lazy dog. ".repeat(40),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::new(20).unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await?;

    assert!(collected.token_results.iter().any(|result| matches!(
        &result.token_result,
        GeneratedTokenResult::PromptExceedsContextSize(details)
            if details.sequence_context_size == SEQUENCE_CONTEXT_SIZE
                && details.prompt_tokens > SEQUENCE_CONTEXT_SIZE as usize
    )));

    cluster.shutdown().await?;

    Ok(())
}
