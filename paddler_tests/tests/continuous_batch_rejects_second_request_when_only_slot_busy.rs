#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_rejects_second_request_when_only_slot_busy() {
    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        max_buffered_requests: 0,
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let mut first_stream = cluster
        .continue_from_raw_prompt_stream(CancellationToken::new(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    let _first_message = first_stream
        .next()
        .await
        .expect("first stream must yield at least one message");

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .expect("first request should occupy the only slot");

    let second_failed = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(10).unwrap(),
                raw_prompt: "Hello".to_owned(),
            },
        )
        .await
        .is_err();

    assert!(
        second_failed,
        "second request must be rejected when the only slot is busy and buffering is disabled"
    );

    drop(first_stream);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
