#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_only_content_tokens_in_usage_for_plain_content() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Say hello.".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: Some(gbnf_literal("Hello there, nice to meet you.")),
                max_tokens: NonZeroU32::new(60).unwrap(),
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    let last = collected
        .token_results
        .last()
        .expect("no token results received");
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        panic!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.prompt_tokens > 0);
    assert!(summary.usage.content_tokens > 0);
    assert_eq!(summary.usage.reasoning_tokens, 0);
    assert_eq!(summary.usage.tool_call_tokens, 0);
    assert_eq!(
        summary.usage.completion_tokens(),
        summary.usage.content_tokens + summary.usage.undeterminable_tokens
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
