use anyhow::Result;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_model_card_fixture::gemma_4_e2b_it::gemma_4_e2b_it;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving Gemma 4 cannot start.
pub async fn start_cluster_with_gemma_4(agents: Vec<AgentConfig>) -> Result<Cluster> {
    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(gemma_4_e2b_it().into_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
