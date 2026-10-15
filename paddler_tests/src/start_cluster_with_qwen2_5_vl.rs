use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen2_5_vl_3b::qwen2_5_vl_3b;
use paddler_test_cluster_harness::model_card::qwen2_5_vl_3b_mmproj::qwen2_5_vl_3b_mmproj;

use crate::start_cluster::start_cluster;

pub async fn start_cluster_with_qwen2_5_vl(agents: Vec<AgentConfig>) -> Result<Cluster> {
    start_cluster(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(
            qwen2_5_vl_3b().into_desired_state_with_multimodal_projection(qwen2_5_vl_3b_mmproj()),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
