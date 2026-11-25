use reqwest::StatusCode;
use reqwest::get;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::constants::API_PATH_INFERENCE_SOCKET;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_rejects_a_request_without_a_websocket_upgrade() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");

    let response = get(format!(
        "http://{}{}",
        cluster.balancer.addresses.inference, API_PATH_INFERENCE_SOCKET
    ))
    .await
    .expect("the balancer must answer a plain HTTP request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
