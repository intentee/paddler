#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn management_reports_zero_download_progress_after_load_complete() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let snapshot = cluster
        .client_management
        .get_agents(CancellationToken::new())
        .await
        .map_err(anyhow::Error::new)
        .context("get_agents should succeed")?;

    assert!(
        !snapshot.agents.is_empty(),
        "cluster should have at least one registered agent"
    );

    let agent = &snapshot.agents[0];

    assert_eq!(
        agent.download_current, 0,
        "download_current should be 0 once the model is loaded"
    );
    assert_eq!(
        agent.download_total, 0,
        "download_total should be 0 once the model is loaded"
    );

    cluster.shutdown().await?;

    Ok(())
}
