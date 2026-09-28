use anyhow::Result;

use crate::smolvlm2_desired_state::smolvlm2_desired_state;
use crate::start_cluster::start_cluster;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

pub async fn start_cluster_with_smolvlm2(agents: Vec<AgentConfig>) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents,
        desired_state: Some(smolvlm2_desired_state()),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
