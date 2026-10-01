#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::unending_grammar::unending_grammar;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(20).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_completion_tokens_matching_streamed_token_count() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Tell me a long story.".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: Some(unending_grammar()),
                max_tokens: MAX_TOKENS,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    let streamed_token_count = collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count() as u64;

    let last = collected
        .token_results
        .last()
        .expect("no token results received");
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        panic!("last result was not Done: {last:?}");
    };

    assert_eq!(streamed_token_count, u64::from(MAX_TOKENS.get()));
    assert_eq!(
        summary.usage.completion_tokens(),
        streamed_token_count,
        "Done.usage.completion_tokens must match the count of streamed token deltas"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
