use std::num::NonZeroU32;

use anyhow::Result;

use paddler_balancer_desired_state_fixture::smolvlm2_desired_state::smolvlm2_desired_state;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving `SmolVLM2` with the given context size cannot start.
pub async fn start_cluster_with_smolvlm2_and_context_size(
    agents: Vec<AgentConfig>,
    context_size: NonZeroU32,
) -> Result<Cluster> {
    let desired_state = smolvlm2_desired_state();

    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: ModelRuntimeParameters {
                context_size,
                n_batch: BatchSize::default().at_most(context_size),
                ..desired_state.model_runtime_parameters
            },
            ..desired_state
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
