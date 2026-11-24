use tokio_util::sync::CancellationToken;

use paddler_client::reports_health::ReportsHealth as _;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_health_returns_ok() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");

    let health = cluster
        .compat_typesafe_health_client()
        .expect("a decision cluster must serve TypeSafe compatibility")
        .get_health(CancellationToken::new())
        .await
        .expect("failed to GET TypeSafe compat /health");

    assert_eq!(health, "OK");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
