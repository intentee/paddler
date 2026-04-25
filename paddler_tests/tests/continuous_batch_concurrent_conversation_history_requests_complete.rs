#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

fn user_message(text: &str) -> ConversationMessage {
    ConversationMessage {
        content: ConversationMessageContent::Text(text.to_owned()),
        role: "user".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_concurrent_conversation_history_requests_complete() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(2).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream_a = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![user_message("What is 2+2?")]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 20,
            tools: vec![],
        })
        .await?;

    let stream_b = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![user_message("Name a color")]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 20,
            tools: vec![],
        })
        .await?;

    let (results_a, results_b) = tokio::join!(
        collect_generated_tokens(stream_a),
        collect_generated_tokens(stream_b),
    );

    let collected_a = results_a?;
    let collected_b = results_b?;

    let tokens_a = collected_a
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();
    let tokens_b = collected_b
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(tokens_a > 0);
    assert!(tokens_b > 0);
    assert!(matches!(
        collected_a.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));
    assert!(matches!(
        collected_b.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    cluster.shutdown().await?;

    Ok(())
}
