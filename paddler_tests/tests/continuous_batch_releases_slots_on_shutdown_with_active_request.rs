#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_releases_slots_on_shutdown_with_active_request() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(1).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 500,
            raw_prompt: "Write a long essay".to_owned(),
        })
        .await?;

    let _first_message = stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("inference stream must yield at least one message"))?;

    drop(stream);

    cluster.shutdown().await?;

    Ok(())
}
