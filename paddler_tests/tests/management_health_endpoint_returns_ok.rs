use anyhow::Context as _;
use anyhow::Result;
use paddler_client::reports_health::ReportsHealth as _;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn management_health_endpoint_returns_ok() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .context("failed to start subprocess cluster")?;

    let health = cluster
        .client_management
        .get_health()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to GET /health")?;

    assert_eq!(health, "OK");

    cluster.shutdown().await?;

    Ok(())
}
