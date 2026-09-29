use std::num::NonZeroU32;

use std::time::Duration;

use futures_util::StreamExt as _;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_504_when_no_agents_registered() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 1,
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    let message = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(10).unwrap(),
                raw_prompt: "Hello".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted")
        .next()
        .await
        .expect("the inference stream must yield a message")
        .expect("the inference stream message must be readable");

    assert!(matches!(message, Message::Error(envelope) if envelope.error.code == 504));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
