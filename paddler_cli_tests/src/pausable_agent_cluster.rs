use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::pausable_agent::PausableAgent;
use crate::pausable_agent_cluster_params::PausableAgentClusterParams;
use crate::pausable_agent_params::PausableAgentParams;
use crate::start_subprocess_cluster::start_subprocess_cluster;

pub struct PausableAgentCluster {
    pub cluster: Cluster,
    pub pausable_agent: PausableAgent,
}

impl PausableAgentCluster {
    /// # Errors
    ///
    /// Fails when the subprocess cluster does not start, when the pausable agent cannot be spawned, and when it does not join the cluster with its expected slots.
    pub async fn start(
        PausableAgentClusterParams {
            binary_path,
            desired_state,
            expected_slots_total,
            slot_count,
        }: PausableAgentClusterParams,
    ) -> Result<Self> {
        let mut cluster = start_subprocess_cluster(
            &binary_path,
            ClusterParams {
                desired_state,
                ..ClusterParams::without_agents()
            },
        )
        .await?;
        let management_addr = cluster.balancer.addresses.management;
        let pausable_agent = PausableAgent::join(
            &mut cluster,
            PausableAgentParams {
                binary_path,
                config: AgentConfig::single(slot_count),
                expected_slots_total,
                management_addr,
            },
        )
        .await?;

        Ok(Self {
            cluster,
            pausable_agent,
        })
    }
}
