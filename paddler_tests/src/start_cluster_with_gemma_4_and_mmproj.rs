use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::gemma_4_e2b_it::gemma_4_e2b_it;
use paddler_test_cluster_harness::model_card::gemma_4_e2b_it_mmproj::gemma_4_e2b_it_mmproj;

use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_gemma_4_and_mmproj(agents: Vec<AgentConfig>) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(
            gemma_4_e2b_it().into_desired_state_with_multimodal_projection(gemma_4_e2b_it_mmproj()),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
