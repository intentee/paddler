use std::num::NonZeroU32;

use anyhow::Result;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_model_card_fixture::qwen3_5_0_8b_mmproj::qwen3_5_0_8b_mmproj;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving Qwen3.5 with the given context size cannot start.
pub async fn start_cluster_with_qwen3_5_and_context_size(
    agents: Vec<AgentConfig>,
    with_mmproj: bool,
    context_size: NonZeroU32,
) -> Result<Cluster> {
    let base_desired_state = if with_mmproj {
        qwen3_5_0_8b().into_desired_state_with_multimodal_projection(qwen3_5_0_8b_mmproj())
    } else {
        qwen3_5_0_8b().into_desired_state()
    };

    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: ModelRuntimeParameters {
                context_size,
                n_batch: BatchSize::default().at_most(context_size),
                ..base_desired_state.model_runtime_parameters
            },
            ..base_desired_state
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
