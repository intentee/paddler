#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_streams_tokens_from_conversation_history_over_http() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Say hello".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: true,
                grammar: None,
                max_tokens: 50,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(token_count > 0);

    cluster.shutdown().await?;

    Ok(())
}
