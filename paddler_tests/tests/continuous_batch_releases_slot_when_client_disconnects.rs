#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_releases_slot_when_client_disconnects() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let agent_id = cluster
        .agents
        .first()
        .map(|agent| agent.id.clone())
        .context("cluster must have one registered agent")?;

    let mut stream = cluster
        .inference_client
        .http()
        .continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 500,
            raw_prompt: "Write a long story about an explorer".to_owned(),
        })
        .await?;

    let first_message = stream
        .next()
        .await
        .context("inference stream must produce at least one message")?;

    drop(first_message);

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .context("first request should occupy the only slot")?;

    drop(stream);

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .context("slot should be released after the HTTP client disconnects")?;

    cluster.shutdown().await?;

    Ok(())
}
