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
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_request_after_agent_with_capacity_registers() {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 10,
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let mut early_stream = cluster
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

    let early_message = early_stream
        .next()
        .await
        .expect("inference stream must yield a message")
        .expect("the message must be readable");

    assert!(matches!(
        early_message,
        Message::Error(envelope) if envelope.error.code == 504
    ));

    cluster
        .spawn_additional_agent(&AgentConfig {
            name: "capacity-agent".to_owned(),
            slot_count: 4,
        })
        .expect("the additional agent must start");

    cluster
        .agents_watcher
        .until(|snapshot| {
            snapshot.agents.len() == 1
                && snapshot
                    .agents
                    .iter()
                    .any(|agent| agent.status.runtime.slots_total() >= 4)
        })
        .await
        .expect("agent should register with 4 slots");

    let later_stream = cluster
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

    collect_generated_tokens(later_stream)
        .await
        .expect("a request must be served once an agent with capacity registered")
        .summary()
        .expect("the served request must finish with a summary");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
