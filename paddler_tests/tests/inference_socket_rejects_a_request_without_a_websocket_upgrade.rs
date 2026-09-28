use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use reqwest::StatusCode;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_rejects_a_request_without_a_websocket_upgrade() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    let response = reqwest::get(format!(
        "http://{}/api/v1/inference_socket",
        cluster.balancer.addresses.inference
    ))
    .await
    .expect("the balancer must answer a plain HTTP request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
