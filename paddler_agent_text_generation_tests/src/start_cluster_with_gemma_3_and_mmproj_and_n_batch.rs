use anyhow::Result;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::gemma_3_4b_it::gemma_3_4b_it;
use paddler_model_card_fixture::gemma_3_4b_it_mmproj::gemma_3_4b_it_mmproj;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving Gemma 3 with its multimodal projection cannot start.
pub async fn start_cluster_with_gemma_3_and_mmproj_and_n_batch(
    agents: Vec<AgentConfig>,
    n_batch: BatchSize,
) -> Result<Cluster> {
    let base_desired_state =
        gemma_3_4b_it().into_desired_state_with_multimodal_projection(gemma_3_4b_it_mmproj());

    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                n_batch: ModelDefaultOr::Explicit(n_batch),
                ..base_desired_state.model_runtime_parameters
            },
            ..base_desired_state
        })),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
