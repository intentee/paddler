use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Message;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_answers_a_ping_with_a_pong() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");
    let mut socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the raw inference socket must connect");

    socket
        .send(Message::Ping(Bytes::from_static(b"probe")))
        .await
        .expect("the ping must be sent");

    assert_eq!(
        socket
            .next_pong()
            .await
            .expect("the balancer must answer the ping"),
        Bytes::from_static(b"probe")
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
