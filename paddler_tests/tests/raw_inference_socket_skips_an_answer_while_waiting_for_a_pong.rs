use serde_json::json;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Message;

use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::start_cluster::start_cluster;

const PING_PAYLOAD: &[u8] = b"probe";

#[tokio::test(flavor = "multi_thread")]
async fn raw_inference_socket_skips_an_answer_while_waiting_for_a_pong() {
    let cluster = start_cluster(ClusterParams::without_agents())
        .await
        .expect("a cluster without agents must start");
    let mut socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the raw inference socket must connect");

    socket
        .send(Message::text(
            json!({
                "Request": {
                    "id": "request-answered-before-the-pong",
                    "request": { "NoSuchRequest": {} }
                }
            })
            .to_string(),
        ))
        .await
        .expect("the undeserializable request must be sent");
    socket
        .send(Message::Ping(Bytes::from_static(PING_PAYLOAD)))
        .await
        .expect("the ping must be sent");

    assert_eq!(
        socket
            .next_pong()
            .await
            .expect("the pong must arrive after the answer"),
        Bytes::from_static(PING_PAYLOAD)
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
