#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;

const SHARED_REQUEST_ID: &str = "shared-request-id";
const MAX_TOKENS_THAT_KEEP_THE_FIRST_REQUEST_GENERATING: NonZeroU32 =
    NonZeroU32::new(2048).unwrap();

const fn is_generated_token(message: &InferenceClientMessage) -> bool {
    matches!(
        message,
        InferenceClientMessage::Response(ResponseEnvelope {
            response: Response::GeneratedToken(generated_token_result),
            ..
        }) if generated_token_result.is_token()
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_requests_from_different_connections_that_share_a_request_id() {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 2),
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("a two-slot qwen3 cluster must start");
    let mut first_socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the first socket must connect");
    let mut second_socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the second socket must connect");

    first_socket
        .send_raw_prompt_request(
            SHARED_REQUEST_ID,
            MAX_TOKENS_THAT_KEEP_THE_FIRST_REQUEST_GENERATING,
        )
        .await
        .expect("the first request must be sent");

    let first_answer = first_socket
        .next_answer_to(SHARED_REQUEST_ID)
        .await
        .expect("the first request must start streaming");

    second_socket
        .send_raw_prompt_request(
            SHARED_REQUEST_ID,
            MAX_TOKENS_THAT_KEEP_THE_FIRST_REQUEST_GENERATING,
        )
        .await
        .expect("the second request must be sent");

    let second_answer = second_socket
        .next_answer_to(SHARED_REQUEST_ID)
        .await
        .expect("the second request must be answered");

    assert!(is_generated_token(&first_answer));
    assert!(is_generated_token(&second_answer));

    drop(first_socket);
    drop(second_socket);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
