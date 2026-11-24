use reqwest::StatusCode;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::noul_system_one_request::noul_system_one_request;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_system_one_rejects_a_request_without_a_model() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a balancer without agents must start");

    let mut request_without_a_model = noul_system_one_request();

    request_without_a_model
        .as_object_mut()
        .expect("the System One request must be a JSON object")
        .remove("model");

    let response = cluster
        .typesafe_system_one("without-a-model", &request_without_a_model)
        .await
        .expect("the request must be answered");

    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
