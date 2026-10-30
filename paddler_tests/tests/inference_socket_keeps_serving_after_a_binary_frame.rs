use std::num::NonZeroU32;

use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::short_buffered_request_timeout::SHORT_BUFFERED_REQUEST_TIMEOUT;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_keeps_serving_after_a_binary_frame() {
    let cluster = start_cluster(ClusterParams {
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        max_buffered_requests: 1,
        ..ClusterParams::without_agents()
    })
    .await
    .expect("a cluster without agents must start");
    let mut socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the raw inference socket must connect");

    socket
        .send(Message::Binary(Bytes::from_static(b"\x00\x01")))
        .await
        .expect("the frame must be sent");
    socket
        .send_raw_prompt_request("request-after-binary-frame", NonZeroU32::new(1).unwrap())
        .await
        .expect("the follow-up request must be sent");

    assert!(matches!(
        socket
            .next_answer_to("request-after-binary-frame")
            .await
            .expect("the follow-up request must be answered"),
        InferenceClientMessage::Error(envelope) if envelope.error.code == 504
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
