use anyhow::Result;

use paddler_balancer_desired_state_fixture::smolvlm2_desired_state::smolvlm2_desired_state;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving `SmolVLM2` cannot start.
pub async fn start_cluster_with_smolvlm2(agents: Vec<AgentConfig>) -> Result<Cluster> {
    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(smolvlm2_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
