use anyhow::Result;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::start_cluster::start_cluster;

pub async fn start_single_agent_cluster_with_desired_state(
    desired_state: BalancerDesiredState,
) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, 1),
        desired_state: Some(desired_state),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
}
