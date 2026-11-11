use paddler_balancer::constants::TYPESAFE_HEADER_REQUEST_ID;
use paddler_tests::cluster_without_agents_serving_decisions::cluster_without_agents_serving_decisions;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn typesafe_models_responds_with_a_request_id() {
    let cluster = start_cluster(cluster_without_agents_serving_decisions())
        .await
        .expect("the cluster must start");
    let response = cluster
        .typesafe_models()
        .await
        .expect("the models request must be answered");

    assert!(
        response
            .headers
            .get(TYPESAFE_HEADER_REQUEST_ID)
            .is_some_and(|request_id| !request_id.is_empty())
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
