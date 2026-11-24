use tokio_util::sync::CancellationToken;

use paddler_client::reports_health::ReportsHealth as _;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_inference_health_returns_ok() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("the cluster must start");

    let health = cluster
        .client_inference
        .get_health(CancellationToken::new())
        .await
        .expect("failed to GET inference /health");

    assert_eq!(health, "OK");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
