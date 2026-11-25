#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

const PARTIAL_GPU_LAYER_COUNT: i32 = 14;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_partially_offloaded_to_the_gpu_generates_tokens() {
    let model_runtime_parameters = DesiredModelRuntimeParameters {
        n_gpu_layers: ModelDefaultOr::Explicit(PARTIAL_GPU_LAYER_COUNT),
        ..DesiredModelRuntimeParameters::default()
    };

    let cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig {
            name: "test-agent".to_owned(),
            slot_count: 1,
        }],
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters,
            ..qwen3_0_6b().into_desired_state()
        })),
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
                max_tokens: NonZeroU32::new(16).unwrap(),
                raw_prompt: "Count from 1 to 5:".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    let token_count = collected
        .token_results
        .iter()
        .filter(|TokenResultWithProducer { token_result, .. }| token_result.is_token())
        .count();

    assert!(token_count > 0);
    collected
        .summary()
        .expect("the generation must finish with a summary");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
