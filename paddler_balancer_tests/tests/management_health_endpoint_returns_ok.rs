use tokio_util::sync::CancellationToken;

use paddler_client::reports_health::ReportsHealth as _;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn management_health_endpoint_returns_ok() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("failed to start subprocess cluster");

    let health = cluster
        .client_management
        .get_health(CancellationToken::new())
        .await
        .expect("failed to GET /health");

    assert_eq!(health, "OK");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
