use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn management_metrics_endpoint_exposes_prometheus_gauges() -> Result<()> {
    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;

    let metrics = cluster
        .management_client
        .metrics()
        .await
        .map_err(anyhow::Error::new)
        .context("get_metrics should succeed")?;

    assert!(
        metrics.contains("slots_processing"),
        "metrics must contain slots_processing gauge"
    );
    assert!(
        metrics.contains("slots_total"),
        "metrics must contain slots_total gauge"
    );

    cluster.shutdown().await?;

    Ok(())
}
