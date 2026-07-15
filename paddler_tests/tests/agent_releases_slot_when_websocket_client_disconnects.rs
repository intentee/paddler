#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn agent_releases_slot_when_websocket_client_disconnects() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1)).await?;

    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let mut stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 200,
                raw_prompt: "Write a long story about an explorer".to_owned(),
            },
        )
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
