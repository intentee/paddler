use anyhow::Result;

use paddler_balancer_desired_state_fixture::smolvlm2_desired_state::smolvlm2_desired_state;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving `SmolVLM2` with the given batch size cannot start.
pub async fn start_cluster_with_smolvlm2_and_n_batch(
    agents: Vec<AgentConfig>,
    n_batch: BatchSize,
) -> Result<Cluster> {
    let desired_state = smolvlm2_desired_state();

    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                n_batch: ModelDefaultOr::Explicit(n_batch),
                ..desired_state.model_runtime_parameters
            },
            ..desired_state
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
