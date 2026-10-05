use tokio::sync::broadcast;
use tokio::task::yield_now;

use paddler_client::error::Error;
use paddler_client::inference_socket::connection::Connection;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_connection_rejects_requests_after_the_balancer_closes_it() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");
    let (notification_tx, _notification_rx) = broadcast::channel(1);
    let connection = Connection::connect(
        cluster
            .balancer
            .inference_base_url()
            .expect("the inference service must have a base URL"),
        notification_tx,
    )
    .await
    .expect("the connection to the balancer must open");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    while !connection.is_disconnected() {
        yield_now().await;
    }

    assert!(matches!(
        connection.send("request-after-close".to_owned(), "{}".to_owned()),
        Err(Error::ConnectionDropped { request_id }) if request_id == "request-after-close"
    ));
}
