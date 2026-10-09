#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_completes_buffered_request_after_agent_joins() {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        max_buffered_requests: 1,
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

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

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("balancer should buffer the request before any agent joins");

    cluster
        .spawn_additional_agent(&AgentConfig {
            name: "buffered-agent".to_owned(),
            slot_count: 4,
        })
        .expect("the additional agent must start");

    collect_generated_tokens(stream)
        .await
        .expect("the buffered request must be served once the agent joins")
        .summary()
        .expect("the buffered request must finish with a summary");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
