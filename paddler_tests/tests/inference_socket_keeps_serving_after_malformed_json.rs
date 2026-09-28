use std::time::Duration;

use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::start_cluster::start_cluster;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_keeps_serving_after_malformed_json() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 1,
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");
    let mut socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the raw inference socket must connect");

    socket
        .send(Message::text("not json"))
        .await
        .expect("the frame must be sent");
    socket
        .send_raw_prompt_request("request-after-malformed-json", 1)
        .await
        .expect("the follow-up request must be sent");

    assert!(matches!(
        socket
            .next_answer_to("request-after-malformed-json")
            .await
            .expect("the follow-up request must be answered"),
        InferenceClientMessage::Error(envelope) if envelope.error.code == 504
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
