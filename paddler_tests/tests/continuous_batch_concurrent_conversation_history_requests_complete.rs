#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

fn user_message(text: &str) -> ConversationMessage {
    ConversationMessage {
        content: ConversationMessageContent::Text(text.to_owned()),
        role: "user".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_concurrent_conversation_history_requests_complete() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)])
        .await
        .expect("the cluster must start");

    let params_a = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![user_message("What is 2+2?")]),
        enable_thinking: false,
        grammar: None,
        max_tokens: NonZeroU32::new(20).unwrap(),
        parse_tool_calls: false,
        tools: vec![],
    };
    let params_b = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![user_message("Name a color")]),
        enable_thinking: false,
        grammar: None,
        max_tokens: NonZeroU32::new(20).unwrap(),
        parse_tool_calls: false,
        tools: vec![],
    };
    let (results_a, results_b) = join!(
        cluster.continue_from_conversation_history(CancellationToken::new(), &params_a),
        cluster.continue_from_conversation_history(CancellationToken::new(), &params_b),
    );

    let collected_a = results_a.expect("the first request must complete");
    let collected_b = results_b.expect("the second request must complete");

    let tokens_a = collected_a
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();
    let tokens_b = collected_b
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(tokens_a > 0);
    assert!(tokens_b > 0);
    assert!(matches!(
        collected_a.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));
    assert!(matches!(
        collected_b.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
