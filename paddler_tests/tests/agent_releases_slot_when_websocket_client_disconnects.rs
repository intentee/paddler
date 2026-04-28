#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::agents_status::assert_slots_processing::assert_slots_processing;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_types::request_params::ContinueFromRawPromptParams;
use reqwest::Client;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_releases_slot_when_websocket_client_disconnects() -> Result<()> {
    let mut cluster = start_subprocess_cluster_with_qwen3(1, 1).await?;

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
            max_tokens: 200,
            raw_prompt: "Write a long story about an explorer".to_owned(),
        })
        .await?;

    let _first = stream
        .next()
        .await
        .context("stream must yield at least one message")?;

    cluster
        .agents
        .until(assert_slots_processing(&agent_id, 1))
        .await
        .context("agent should report slot in use")?;

    drop(stream);

    cluster
        .agents
        .until(assert_slots_processing(&agent_id, 0))
        .await
        .context("agent should release slot after the client disconnects")?;

    cluster.shutdown().await?;

    Ok(())
}
