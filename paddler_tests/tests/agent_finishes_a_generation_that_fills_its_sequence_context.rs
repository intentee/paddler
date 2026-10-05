#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_finishes_a_generation_that_fills_its_sequence_context() {
    let base_desired_state = qwen3_0_6b().into_desired_state();

    let cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(1)],
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                n_batch: BatchSize::try_from(256).expect("the value must fit its target type"),
                context_size: NonZeroU32::try_from(256)
                    .expect("the value must fit its target type"),
                ..base_desired_state.inference_parameters
            },
            ..base_desired_state
        }),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .continue_from_raw_prompt(CancellationToken::new(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    assert_eq!(
        collected
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::ContextFull
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
