use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::gemma_3_4b_it::gemma_3_4b_it;

use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_gemma_3(agents: Vec<AgentConfig>) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents,
        desired_state: Some(gemma_3_4b_it().into_desired_state()),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
