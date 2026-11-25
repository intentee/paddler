use anyhow::Result;

use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving Qwen3 cannot start.
pub async fn start_cluster_with_qwen3(agents: Vec<AgentConfig>) -> Result<Cluster> {
    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
