#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;
use std::time::Duration;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_504_when_inference_item_timeout_is_zero() {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 2),
        inference_item_timeout: Duration::ZERO,
        wait_for_slots_ready: true,
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let mut stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(10).unwrap(),
                raw_prompt: "Hello".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    let message = stream
        .next()
        .await
        .expect("inference stream must yield a message")
        .expect("the message must be readable");

    assert!(matches!(
        message,
        Message::Error(envelope) if envelope.error.code == 504
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
