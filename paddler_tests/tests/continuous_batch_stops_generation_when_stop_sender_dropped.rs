#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_stops_generation_when_stop_sender_dropped() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(2).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut first_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 500,
            raw_prompt: "Write a long essay about photosynthesis".to_owned(),
        })
        .await?;

    let _first_token = first_stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("first stream must yield at least one message"))?;

    drop(first_stream);

    let second_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "Hello".to_owned(),
        })
        .await?;

    let second_collected = collect_generated_tokens(second_stream).await?;

    assert!(matches!(
        second_collected.token_results.last(),
        Some(GeneratedTokenResult::Done)
    ));

    let second_token_count = second_collected
        .token_results
        .iter()
        .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
        .count();

    assert!(
        second_token_count > 0,
        "second sequential request must succeed after first stream is dropped"
    );

    cluster.shutdown().await?;

    Ok(())
}
