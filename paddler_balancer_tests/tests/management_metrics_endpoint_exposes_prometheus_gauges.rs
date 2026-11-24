use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn management_metrics_endpoint_exposes_prometheus_gauges() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("the cluster must start");

    let metrics = cluster
        .client_management
        .get_metrics(CancellationToken::new())
        .await
        .expect("get_metrics should succeed");

    assert!(
        metrics.contains("slots_processing"),
        "metrics must contain slots_processing gauge"
    );
    assert!(
        metrics.contains("slots_total"),
        "metrics must contain slots_total gauge"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
