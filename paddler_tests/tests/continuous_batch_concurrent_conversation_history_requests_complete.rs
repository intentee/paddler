#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

fn user_message(text: &str) -> ConversationMessage {
    ConversationMessage {
        content: ConversationMessageContent::Text(text.to_owned()),
        role: "user".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_concurrent_conversation_history_requests_complete() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)]).await?;

    let params_a = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![user_message("What is 2+2?")]),
        enable_thinking: false,
        grammar: None,
        max_tokens: 20,
        parse_tool_calls: false,
        tools: vec![],
    };
    let params_b = ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![user_message("Name a color")]),
        enable_thinking: false,
        grammar: None,
        max_tokens: 20,
        parse_tool_calls: false,
        tools: vec![],
    };
    let (results_a, results_b) = tokio::join!(
        cluster.continue_from_conversation_history(CancellationToken::new(), &params_a),
        cluster.continue_from_conversation_history(CancellationToken::new(), &params_b),
    );

    let collected_a = results_a?;
    let collected_b = results_b?;

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

    cluster.shutdown().await?;

    Ok(())
}
