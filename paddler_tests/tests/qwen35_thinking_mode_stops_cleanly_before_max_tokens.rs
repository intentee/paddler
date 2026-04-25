#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3_5::start_in_process_cluster_with_qwen3_5;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn qwen35_thinking_mode_stops_cleanly_before_max_tokens() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3_5(1, false).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let stream = inference_client
        .post_continue_from_conversation_history(&ContinueFromConversationHistoryParams {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Text("What is 2+2?".to_owned()),
                role: "user".to_owned(),
            }]),
            enable_thinking: true,
            grammar: None,
            max_tokens: 2000,
            tools: vec![],
        })
        .await?;

    let collected = collect_generated_tokens(stream).await?;

    let token_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(token_count > 0);
    assert!(token_count <= 2000);
    assert!(matches!(
        collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    cluster.shutdown().await?;

    Ok(())
}
