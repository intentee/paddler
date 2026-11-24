use anyhow::Result;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_model_card_fixture::ministral_3_8b_reasoning::ministral_3_8b_reasoning;
use paddler_model_card_fixture::ministral_3_8b_reasoning_mmproj::ministral_3_8b_reasoning_mmproj;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving Ministral 3 with its multimodal projection cannot start.
pub async fn start_cluster_with_ministral_3_and_mmproj(
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(
            ministral_3_8b_reasoning()
                .into_desired_state_with_multimodal_projection(ministral_3_8b_reasoning_mmproj()),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
