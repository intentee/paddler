use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_inference_health_returns_ok() -> Result<()> {
    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;

    assert_eq!(cluster.inference_client.health().await?, "OK");

    cluster.shutdown().await?;

    Ok(())
}
