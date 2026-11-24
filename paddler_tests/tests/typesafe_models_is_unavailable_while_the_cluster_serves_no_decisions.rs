use reqwest::StatusCode;

use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_models_is_unavailable_while_the_cluster_serves_no_decisions() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");

    let response = cluster
        .typesafe_models()
        .await
        .expect("the models request must be answered");

    assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
