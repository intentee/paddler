#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_health_endpoint_returns_ok() -> Result<()> {
    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await
    .context("failed to start subprocess cluster")?;

    let health = cluster
        .paddler_client
        .management()
        .get_health()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to GET /health")?;

    assert_eq!(health, "OK");

    cluster.shutdown().await?;

    Ok(())
}
