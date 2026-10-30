use std::num::NonZeroU32;

use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::short_buffered_request_timeout::SHORT_BUFFERED_REQUEST_TIMEOUT;
use paddler_tests::start_cluster::start_cluster;

const AWAITED_REQUEST_ID: &str = "request-after-the-ping";

#[tokio::test(flavor = "multi_thread")]
async fn raw_inference_socket_skips_a_pong_while_waiting_for_an_answer() {
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
        .send(Message::Ping(Bytes::from_static(b"probe")))
        .await
        .expect("the ping must be sent");
    socket
        .send_raw_prompt_request(AWAITED_REQUEST_ID, NonZeroU32::MIN)
        .await
        .expect("the request must be sent");

    assert!(matches!(
        socket
            .next_answer_to(AWAITED_REQUEST_ID)
            .await
            .expect("the request must be answered after the pong"),
        InferenceClientMessage::Error(ErrorEnvelope {
            request_id,
            error: JsonRpcError { code: 504, .. },
        }) if request_id == AWAITED_REQUEST_ID
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
