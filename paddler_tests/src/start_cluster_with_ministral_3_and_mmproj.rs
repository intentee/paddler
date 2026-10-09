use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ministral_3_8b_reasoning::ministral_3_8b_reasoning;
use paddler_test_cluster_harness::model_card::ministral_3_8b_reasoning_mmproj::ministral_3_8b_reasoning_mmproj;

use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_ministral_3_and_mmproj(
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(
            ministral_3_8b_reasoning()
                .into_desired_state_with_multimodal_projection(ministral_3_8b_reasoning_mmproj()),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
