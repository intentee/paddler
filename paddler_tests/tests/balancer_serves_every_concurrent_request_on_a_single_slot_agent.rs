#![cfg(feature = "tests_that_use_llms")]

use std::iter::repeat_with;
use std::num::NonZeroU32;

use futures_util::future::try_join_all;
use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;

const CONCURRENT_REQUESTS: usize = 256;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_every_concurrent_request_on_a_single_slot_agent() {
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
        max_buffered_requests: u64::try_from(CONCURRENT_REQUESTS)
            .expect("the value must fit its target type"),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected_requests = try_join_all(
        repeat_with(|| async {
            collect_generated_tokens(
                cluster
                    .client_inference
                    .continue_from_raw_prompt(
                        CancellationToken::new(),
                        ContinueFromRawPromptParams {
                            grammar: None,
                            max_tokens: NonZeroU32::new(1).unwrap(),
                            raw_prompt: "Hello".to_owned(),
                        },
                    )
                    .await
                    .expect("the inference request must be accepted"),
            )
            .await
        })
        .take(CONCURRENT_REQUESTS),
    )
    .await
    .expect("every concurrent request must succeed");

    for collected in collected_requests {
        assert!(matches!(
            collected
                .token_results
                .last()
                .map(|token_result_with_producer| &token_result_with_producer.token_result),
            Some(GeneratedTokenResult::Done(_))
        ));
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
