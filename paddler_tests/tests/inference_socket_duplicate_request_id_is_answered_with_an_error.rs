use std::num::NonZeroU32;

use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::start_cluster::start_cluster;

const DUPLICATE_REQUEST_ID: &str = "duplicate-request-id";

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_duplicate_request_id_is_answered_with_an_error() {
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
        .send_raw_prompt_request(DUPLICATE_REQUEST_ID, NonZeroU32::MIN)
        .await
        .expect("the first request must be sent");
    socket
        .send_raw_prompt_request(DUPLICATE_REQUEST_ID, NonZeroU32::MIN)
        .await
        .expect("the duplicate request must be sent");

    assert!(matches!(
        socket
            .next_answer_to(DUPLICATE_REQUEST_ID)
            .await
            .expect("a duplicate request id must be answered instead of leaving the client waiting forever"),
        InferenceClientMessage::Error(envelope) if envelope.error.code == 400
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
