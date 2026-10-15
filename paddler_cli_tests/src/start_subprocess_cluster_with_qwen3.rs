use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;

use crate::start_subprocess_cluster::start_subprocess_cluster;

pub async fn start_subprocess_cluster_with_qwen3(
    binary_path: &str,
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    start_subprocess_cluster(
        binary_path,
        ClusterParams {
            agents,
            desired_state: ClusterDesiredState::Apply(Box::new(qwen3_0_6b().into_desired_state())),
            wait_for_slots_ready: true,
            ..ClusterParams::default()
        },
    )
    .await
}
