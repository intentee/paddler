#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_with_thinking_enabled_emits_reasoning_tokens() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text(
                    "What is two plus two? Think step by step.".to_owned(),
                ),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 600,
            parse_tool_calls: false,
            tools: vec![],
        })
        .await?;

    let reasoning_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result.token_result, GeneratedTokenResult::ReasoningToken(_)))
        .count();

    assert!(
        reasoning_count > 0,
        "expected at least one reasoning token when thinking is enabled (got {reasoning_count})"
    );

    let last = collected
        .token_results
        .last()
        .ok_or_else(|| anyhow::anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.prompt_tokens > 0);
    assert!(summary.usage.reasoning_tokens > 0);
    assert_eq!(
        summary.usage.completion_tokens(),
        summary.usage.content_tokens
            + summary.usage.reasoning_tokens
            + summary.usage.undeterminable_tokens
    );

    cluster.shutdown().await?;

    Ok(())
}
