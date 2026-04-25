#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::agents_status::AgentsStatus;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_stop_signal_terminates_generation_before_max_tokens() -> Result<()> {
    let mut cluster = start_in_process_cluster_with_qwen3(1).await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 500,
            raw_prompt: "Write a very long story about a dragon".to_owned(),
        })
        .await?;

    let _first_message = stream
        .next()
        .await
        .context("inference stream must yield at least one message")?;

    cluster
        .agents
        .until(AgentsStatus::slots_processing_is(&agent_id, 1))
        .await
        .context("slot should be occupied while the request is in flight")?;

    drop(stream);

    cluster
        .agents
        .until(AgentsStatus::slots_processing_is(&agent_id, 0))
        .await
        .context("dropping the stream must terminate generation before max_tokens is reached")?;

    cluster.shutdown().await?;

    Ok(())
}
