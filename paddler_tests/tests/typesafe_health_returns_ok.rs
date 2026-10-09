use tokio_util::sync::CancellationToken;

use paddler_client::reports_health::ReportsHealth as _;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_health_returns_ok() {
    let cluster = start_cluster(cluster_without_agents_serving(InferenceMode::Decision))
        .await
        .expect("a decision balancer without agents must start");

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
