use anyhow::Context as _;
use anyhow::Result;
use paddler_client::reports_health::ReportsHealth as _;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_inference_health_returns_ok() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let health = cluster
        .client_inference
        .get_health(CancellationToken::new())
        .await
        .context("failed to GET inference /health")?;

    assert_eq!(health, "OK");

    cluster.shutdown().await?;

    Ok(())
}
