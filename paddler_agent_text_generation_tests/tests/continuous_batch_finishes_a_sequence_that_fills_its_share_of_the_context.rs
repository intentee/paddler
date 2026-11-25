#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::unending_generation::unending_generation;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_finishes_a_sequence_that_fills_its_share_of_the_context() {
    let base_desired_state = qwen3_0_6b().into_desired_state();

    let cluster = Cluster::start_in_process(ClusterParams {
        agents: vec![AgentConfig::single(2)],
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                n_batch: ModelDefaultOr::Explicit(
                    BatchSize::try_from(256).expect("the value must fit its target type"),
                ),
                context_size: ModelDefaultOr::Explicit(
                    NonZeroU32::try_from(256).expect("the value must fit its target type"),
                ),
                ..base_desired_state.model_runtime_parameters
            },
            ..base_desired_state
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let bounded_params = ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: NonZeroU32::new(20).unwrap(),
        raw_prompt: "Hi".to_owned(),
    };
    let (context_filling_collected, bounded_collected) = join!(
        cluster.continue_from_raw_prompt(CancellationToken::new(), &unending_generation()),
        cluster.continue_from_raw_prompt(CancellationToken::new(), &bounded_params),
    );

    assert_eq!(
        context_filling_collected
            .expect("the inference request must be accepted")
            .summary()
            .expect("the generation must finish with a summary")
            .finish,
        GenerationFinish::ContextFull
    );
    assert!(
        bounded_collected
            .expect("the message must be readable")
            .summary()
            .is_ok()
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
