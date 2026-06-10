#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use futures_util::future;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_internal_endpoint_concurrent_requests_keep_independent_usage() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)]).await?;

    let prompts = ["Say hi.", "Count to three."];

    let futures = prompts.iter().map(|prompt| {
        let prompt = (*prompt).to_owned();
        let generation =
            cluster.continue_from_conversation_history(&ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(prompt),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: 30,
                parse_tool_calls: false,
                tools: vec![],
            });

        async move {
            let collected = generation.await?;

            let last = collected
                .token_results
                .last()
                .ok_or_else(|| anyhow!("no token results received"))?;
            match &last.token_result {
                GeneratedTokenResult::Done(summary) => {
                    Ok::<GenerationSummary, anyhow::Error>(*summary)
                }
                other => Err(anyhow!("last result was not Done: {other:?}")),
            }
        }
    });

    let summaries: Vec<GenerationSummary> = future::try_join_all(futures).await?;

    assert_eq!(summaries.len(), 2);

    for summary in &summaries {
        assert!(summary.usage.prompt_tokens > 0);
        assert!(summary.usage.completion_tokens() > 0);
    }

    // The two requests have different prompts and different generations;
    // their usage breakdowns must not be byte-identical.
    assert_ne!(
        summaries[0].usage, summaries[1].usage,
        "concurrent requests reported identical usage; counters likely shared"
    );

    cluster.shutdown().await?;

    Ok(())
}
