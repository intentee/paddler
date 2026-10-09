use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::pausable_agent::PausableAgent;
use crate::pausable_agent_cluster::PausableAgentCluster;
use crate::pausable_agent_cluster_params::PausableAgentClusterParams;
use crate::pausable_agent_params::PausableAgentParams;
use crate::start_subprocess_cluster::start_subprocess_cluster;

pub async fn start_pausable_agent_cluster(
    PausableAgentClusterParams {
        binary_path,
        desired_state,
        expected_slots_total,
        slot_count,
    }: PausableAgentClusterParams,
) -> Result<PausableAgentCluster> {
    let mut cluster = start_subprocess_cluster(
        &binary_path,
        ClusterParams {
            agents: Vec::new(),
            desired_state,
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;
    let pausable_agent = PausableAgent::join(
        &mut cluster,
        PausableAgentParams {
            binary_path,
            config: AgentConfig::single(slot_count),
            expected_slots_total,
        },
    )
    .await?;

    Ok(PausableAgentCluster {
        cluster,
        pausable_agent,
    })
}
