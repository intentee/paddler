#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::batch_size_within_context::batch_size_within_context;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;

const SEQUENCE_CONTEXT_SIZE: u32 = 256;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_raw_prompt_exceeding_sequence_context() {
    let desired_state = qwen3_desired_state();
    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                context_size: NonZeroU32::try_from(SEQUENCE_CONTEXT_SIZE)
                    .expect("the value must fit its target type"),
                n_batch: batch_size_within_context(
                    NonZeroU32::try_from(SEQUENCE_CONTEXT_SIZE)
                        .expect("the value must fit its target type"),
                )
                .expect("the batch size must fit the context"),
                ..desired_state.inference_parameters
            },
            ..desired_state
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(20).unwrap(),
                raw_prompt: "The quick brown fox jumps over the lazy dog. ".repeat(40),
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert!(
        !collected
            .token_results
            .iter()
            .any(|result| result.token_result.is_token())
    );
    assert!(collected.token_results.iter().any(|result| matches!(
        &result.token_result,
        GeneratedTokenResult::PromptExceedsContextSize(details)
            if details.sequence_context_size == SEQUENCE_CONTEXT_SIZE
                && details.prompt_tokens > SEQUENCE_CONTEXT_SIZE as usize
    )));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
