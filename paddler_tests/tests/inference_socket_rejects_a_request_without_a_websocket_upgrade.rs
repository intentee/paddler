use reqwest::StatusCode;
use reqwest::get;

use paddler_messaging::api_path::ApiPath;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_rejects_a_request_without_a_websocket_upgrade() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    let response = get(format!(
        "http://{}{}",
        cluster.balancer.addresses.inference,
        ApiPath::INFERENCE_SOCKET
    ))
    .await
    .expect("the balancer must answer a plain HTTP request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
