#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_produces_distinct_outputs_for_concurrent_prompts() {
    let desired_state = qwen3_0_6b().into_desired_state();

    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 2,
        }],
        desired_state: Some(desired_state),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let params_a = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: NonZeroU32::new(20).unwrap(),
        raw_prompt: "Count from one to ten in English: one, two,".to_owned(),
    };
    let params_b = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: NonZeroU32::new(20).unwrap(),
        raw_prompt: "The capital of France is".to_owned(),
    };
    let (collected_a, collected_b) = join!(
        cluster.continue_from_raw_prompt(CancellationToken::new(), &params_a),
        cluster.continue_from_raw_prompt(CancellationToken::new(), &params_b),
    );

    let collected_a = collected_a.expect("the first request must complete");
    let collected_b = collected_b.expect("the second request must complete");

    assert!(
        !collected_a.text.is_empty(),
        "first concurrent prompt should produce tokens"
    );
    assert!(
        !collected_b.text.is_empty(),
        "second concurrent prompt should produce tokens"
    );
    assert_ne!(
        collected_a.text, collected_b.text,
        "two different prompts should produce different outputs"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
