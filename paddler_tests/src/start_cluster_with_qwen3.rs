use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::qwen3_desired_state::qwen3_desired_state;
use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_qwen3(agents: Vec<AgentConfig>) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(qwen3_desired_state())),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
