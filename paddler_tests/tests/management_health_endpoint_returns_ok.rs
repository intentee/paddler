use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn management_health_endpoint_returns_ok() -> Result<()> {
    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .context("failed to start subprocess cluster")?;

    let health = cluster
        .management_client
        .health()
        .await
        .map_err(anyhow::Error::new)
        .context("failed to GET /health")?;

    assert_eq!(health, "OK");

    cluster.shutdown().await?;

    Ok(())
}
