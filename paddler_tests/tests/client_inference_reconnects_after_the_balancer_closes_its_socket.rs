use std::num::NonZeroU32;

use std::num::NonZeroUsize;
use std::time::Duration;

use futures_util::StreamExt as _;
use paddler_balancer::max_websocket_frame_size::MAX_WEBSOCKET_FRAME_SIZE;
use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_client::error::Error;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(1).unwrap();

const fn raw_prompt_params(raw_prompt: String) -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: MAX_TOKENS,
        raw_prompt,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn client_inference_reconnects_after_the_balancer_closes_its_socket() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 1,
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");
    let client_inference = ClientInference::new(ClientInferenceParams {
        inference_socket_pool_size: NonZeroUsize::MIN,
        url: cluster
            .balancer
            .inference_base_url()
            .expect("the inference service must have a base URL"),
    });

    let oversized_request_outcome = client_inference
        .continue_from_raw_prompt(
            CancellationToken::new(),
            raw_prompt_params("x".repeat(MAX_WEBSOCKET_FRAME_SIZE)),
        )
        .await
        .expect("the oversized request must be sent")
        .next()
        .await
        .expect("the oversized request must be answered");
    let follow_up_message = client_inference
        .continue_from_raw_prompt(
            CancellationToken::new(),
            raw_prompt_params("Hello".to_owned()),
        )
        .await
        .expect("the follow-up request must be sent over a new socket")
        .next()
        .await
        .expect("the follow-up request must be answered")
        .expect("the follow-up answer must be readable");

    assert!(matches!(
        oversized_request_outcome,
        Err(Error::ConnectionDropped { .. })
    ));
    assert!(matches!(
        follow_up_message,
        Message::Error(envelope) if envelope.error.code == 504
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
