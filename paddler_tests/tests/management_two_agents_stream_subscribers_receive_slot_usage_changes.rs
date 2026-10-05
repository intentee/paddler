#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn management_two_agents_stream_subscribers_receive_slot_usage_changes() {
    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: Some(qwen3_desired_state()),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have registered one agent")
        .clone();

    let generation_cancellation = CancellationToken::new();
    let token_stream = cluster
        .continue_from_raw_prompt_stream(generation_cancellation.clone(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .expect("agents_stream must emit a snapshot showing slot usage");

    generation_cancellation.cancel();
    collect_generated_tokens(token_stream)
        .await
        .expect("the generated tokens must be collected");

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("agents_stream must emit a snapshot showing the slot was released");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
