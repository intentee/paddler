use std::num::NonZeroU32;

use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_answers_an_undeserializable_request_with_an_error() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        max_buffered_requests: 0,
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let mut socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the connection must be established");

    socket
        .send(Message::text(
            json!({
                "Request": {
                    "id": "undeserializable-request",
                    "request": { "NoSuchRequest": {} }
                }
            })
            .to_string(),
        ))
        .await
        .expect("the message must be sent");
    socket
        .send_raw_prompt_request("request-after-the-undeserializable-one", NonZeroU32::MIN)
        .await
        .expect("the request must be sent");

    assert!(matches!(
        socket.next_answer().await.expect("the inference socket must answer"),
        InferenceClientMessage::Error(ErrorEnvelope {
            request_id,
            error: JsonRpcError { code: 400, .. },
        }) if request_id == "undeserializable-request"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
