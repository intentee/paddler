#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3_5::start_cluster_with_qwen3_5;

#[tokio::test(flavor = "multi_thread")]
async fn qwen35_with_system_message_completes_without_thinking() -> Result<()> {
    let cluster = start_cluster_with_qwen3_5(vec![AgentConfig::single(1)], false).await?;

    let conversation_history = ConversationHistory::new(vec![
        ConversationMessage {
            content: ConversationMessageContent::Text(
                "You are a focused web crawler assistant. Your only job is to decide which links to follow to discover more relevant pages. Respond with JSON only.".to_owned(),
            ),
            role: "system".to_owned(),
        },
        ConversationMessage {
            content: ConversationMessageContent::Text(
                "Goal: \"find all PDF reports\"\n\nPage: https://example.com/reports\n\nFollowable links:\n[0] [Navigation] \"Home\" → /home\n[1] [PrimaryListing] \"Annual Report 2024\" → /reports/annual-2024.pdf\n[2] [PrimaryListing] \"Q3 Financial Summary\" → /reports/q3-summary.pdf\n[3] [Navigation] \"Next Page\" → /reports?page=2".to_owned(),
            ),
            role: "user".to_owned(),
        },
    ]);

    let collected = cluster
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history,
            enable_thinking: false,
            grammar: None,
            max_tokens: 512,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(token_count > 0);
    assert!(matches!(
        collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    cluster.shutdown().await?;

    Ok(())
}
