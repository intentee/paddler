use std::num::NonZeroU32;

use anyhow::Result;

use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving Qwen3 with the given context size cannot start.
pub async fn start_cluster_with_qwen3_and_context_size(
    agents: Vec<AgentConfig>,
    context_size: NonZeroU32,
) -> Result<Cluster> {
    let desired_state = qwen3_desired_state();

    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                context_size: ModelDefaultOr::Explicit(context_size),
                n_batch: ModelDefaultOr::Explicit(BatchSize::default().at_most(context_size)),
                ..desired_state.model_runtime_parameters
            },
            ..desired_state
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
