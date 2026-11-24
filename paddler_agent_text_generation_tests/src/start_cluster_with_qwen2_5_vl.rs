use anyhow::Result;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_model_card_fixture::qwen2_5_vl_3b::qwen2_5_vl_3b;
use paddler_model_card_fixture::qwen2_5_vl_3b_mmproj::qwen2_5_vl_3b_mmproj;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving Qwen2.5 VL cannot start.
pub async fn start_cluster_with_qwen2_5_vl(agents: Vec<AgentConfig>) -> Result<Cluster> {
    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(
            qwen2_5_vl_3b().into_desired_state_with_multimodal_projection(qwen2_5_vl_3b_mmproj()),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
