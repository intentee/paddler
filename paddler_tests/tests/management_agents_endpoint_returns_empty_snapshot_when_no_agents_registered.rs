use anyhow::Context as _;
use anyhow::Result;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn management_agents_endpoint_returns_empty_snapshot_when_no_agents_registered() -> Result<()>
{
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let snapshot = cluster
        .client_management
        .get_agents(CancellationToken::new())
        .await
        .map_err(anyhow::Error::new)
        .context("failed to GET /api/v1/agents")?;

    assert!(snapshot.agents.is_empty());

    cluster.shutdown().await?;

    Ok(())
}
