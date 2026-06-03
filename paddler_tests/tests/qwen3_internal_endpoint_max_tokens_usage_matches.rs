#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

const MAX_TOKENS: i32 = 20;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_max_tokens_usage_matches_streamed_count() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("Tell me a long story.".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: MAX_TOKENS,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let streamed_token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count() as u64;

    let last = collected
        .token_results
        .last()
        .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    assert!(streamed_token_count > 0);
    assert!(streamed_token_count <= MAX_TOKENS as u64);
    assert_eq!(
        summary.usage.completion_tokens(),
        streamed_token_count,
        "Done.usage.completion_tokens must match the count of streamed token deltas"
    );

    cluster.shutdown().await?;

    Ok(())
}
