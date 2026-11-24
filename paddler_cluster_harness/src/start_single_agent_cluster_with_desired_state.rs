use anyhow::Result;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving the desired state with a single agent cannot start.
pub async fn start_single_agent_cluster_with_desired_state(
    desired_state: BalancerDesiredState,
) -> Result<Cluster> {
    Cluster::start_in_process(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: ClusterDesiredState::Apply(Box::new(desired_state)),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
}
