#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_client::token_result_with_producer::TokenResultWithProducer;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_reuses_slot_after_request_completes() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let first_collected = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt_collected(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello world".to_owned(),
        })
        .await?;

    assert!(matches!(
        first_collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    let second_collected = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt_collected(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Goodbye world".to_owned(),
        })
        .await?;

    assert!(matches!(
        second_collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    let second_token_count = second_collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(
        second_token_count > 0,
        "second sequential request must reuse the slot and produce tokens"
    );

    cluster.shutdown().await?;

    Ok(())
}
