use anyhow::Result;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_model_card_fixture::deepseek_r1_distill_llama_8b::deepseek_r1_distill_llama_8b;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving `DeepSeek` R1 Distill Llama 8B cannot start.
pub async fn start_cluster_with_deepseek_r1_distill_llama_8b(
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    Cluster::start_in_process(ClusterParams {
        agents,
        desired_state: ClusterDesiredState::Apply(Box::new(
            deepseek_r1_distill_llama_8b().into_desired_state(),
        )),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
}
