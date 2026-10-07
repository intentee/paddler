#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_buffered_requests_across_two_agents() {
    let cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: vec![
                AgentConfig {
                    name: "distributed-agent-0".to_owned(),
                    slot_count: 2,
                },
                AgentConfig {
                    name: "distributed-agent-1".to_owned(),
                    slot_count: 2,
                },
            ],
            wait_for_slots_ready: true,
            max_buffered_requests: 10,
            desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
            ..ClusterParams::default()
        },
    )
    .await
    .expect("the cluster must start");

    let mut streams = Vec::new();

    for _ in 0..5 {
        let stream = cluster
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

        streams.push(stream);
    }

    let mut successful_responses = 0;

    for mut stream in streams {
        if let Some(item) = stream.next().await {
            match item.expect("the message must be readable") {
                Message::Response(_) => successful_responses += 1,
                Message::Error(envelope) => {
                    panic!(
                        "expected success, got error {}: {}",
                        envelope.error.code, envelope.error.description
                    );
                }
            }
        }
    }

    assert_eq!(successful_responses, 5);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
