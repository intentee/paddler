use std::num::NonZeroU32;

use serde_json::to_string;
use tokio_tungstenite::tungstenite::Message;

use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::short_buffered_request_timeout::SHORT_BUFFERED_REQUEST_TIMEOUT;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_keeps_serving_after_a_stop_request_for_an_unknown_request() {
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
        .send(Message::text(
            to_string(
                &InferenceServerMessage::<ValidatedParametersSchema>::Notification(
                    InferenceServerNotification::StopRespondingTo("unknown-request".to_owned()),
                ),
            )
            .expect("the stop request must serialize"),
        ))
        .await
        .expect("the stop request must be sent");
    socket
        .send_raw_prompt_request("request-after-the-stop-request", NonZeroU32::MIN)
        .await
        .expect("the follow-up request must be sent");

    assert!(matches!(
        socket
            .next_answer_to("request-after-the-stop-request")
            .await
            .expect("the follow-up request must be answered"),
        InferenceClientMessage::Error(envelope) if envelope.error.code == 504
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
