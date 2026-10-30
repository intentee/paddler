use std::mem::discriminant;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_balancer::max_websocket_message_size::MAX_WEBSOCKET_MESSAGE_SIZE;
use paddler_client::client_inference::ClientInference;
use paddler_client::client_inference_params::ClientInferenceParams;
use paddler_client::error::Error;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::short_buffered_request_timeout::SHORT_BUFFERED_REQUEST_TIMEOUT;
use paddler_tests::start_cluster::start_cluster;

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
        buffered_request_timeout: SHORT_BUFFERED_REQUEST_TIMEOUT,
        max_buffered_requests: 1,
        ..ClusterParams::without_agents()
    })
    .await
    .expect("a cluster without agents must start");
    let client_inference = ClientInference::new(ClientInferenceParams {
        inference_socket_pool_size: NonZeroUsize::MIN,
        url: cluster.balancer.inference_base_url.clone(),
    });

    let oversized_request_outcome = client_inference
        .continue_from_raw_prompt(
            CancellationToken::new(),
            raw_prompt_params("x".repeat(MAX_WEBSOCKET_MESSAGE_SIZE)),
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

    assert_eq!(
        oversized_request_outcome.as_ref().err().map(discriminant),
        Some(discriminant(&Error::ConnectionDropped {
            request_id: String::new()
        }))
    );
    assert!(matches!(
        follow_up_message,
        Message::Error(envelope) if envelope.error.code == 504
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
