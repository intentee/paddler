#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_cluster::agent_config::AgentConfig;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_releases_slot_when_websocket_client_disconnects() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1)).await?;

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
            max_tokens: 200,
            raw_prompt: "Write a long story about an explorer".to_owned(),
        })
        .await?;

    let _first = stream
        .next()
        .await
        .context("stream must yield at least one message")?;

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .context("agent should report slot in use")?;

    drop(stream);

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .context("agent should release slot after the client disconnects")?;

    cluster.shutdown().await?;

    Ok(())
}
