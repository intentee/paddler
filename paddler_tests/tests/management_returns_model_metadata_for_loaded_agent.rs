#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn management_returns_model_metadata_for_loaded_agent() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let agent_id = cluster
        .agents
        .first()
        .map(|agent| agent.id.clone())
        .context("cluster must have at least one registered agent")?;

    let metadata = loop {
        match cluster.management_client.model_metadata(&agent_id).await {
            Ok(Some(metadata)) => break metadata,
            _ => tokio::task::yield_now().await,
        }
    };

    assert!(
        !metadata.metadata.is_empty(),
        "model metadata map must not be empty"
    );

    cluster.shutdown().await?;

    Ok(())
}
