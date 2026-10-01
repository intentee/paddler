#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::future;
use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_independent_usage_for_concurrent_requests() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)])
        .await
        .expect("the cluster must start");

    let forced_replies = ["one", "one two three four five"];

    let futures = forced_replies.iter().map(|forced_reply| {
        let generation = cluster.continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Count.".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: Some(gbnf_literal(forced_reply)),
                max_tokens: NonZeroU32::new(30).unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        );

        async move {
            let collected = generation.await.expect("the generation must complete");

            let last = collected
                .token_results
                .last()
                .expect("no token results received");
            match &last.token_result {
                GeneratedTokenResult::Done(summary) => *summary,
                other => panic!("last result was not Done: {other:?}"),
            }
        }
    });

    let summaries: Vec<GenerationSummary> = future::join_all(futures).await;

    assert_eq!(summaries.len(), 2);

    for summary in &summaries {
        assert!(summary.usage.prompt_tokens > 0);
        assert!(summary.usage.completion_tokens() > 0);
    }

    assert_ne!(
        summaries[0].usage, summaries[1].usage,
        "concurrent requests reported identical usage; counters likely shared"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
