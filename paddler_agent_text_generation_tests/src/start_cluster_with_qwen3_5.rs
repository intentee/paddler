use anyhow::Result;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;

use crate::start_cluster_with_qwen3_5_and_context_size::start_cluster_with_qwen3_5_and_context_size;

/// # Errors
///
/// Returns the error of [`Cluster::start_in_process`] when the cluster serving Qwen3.5 cannot start.
pub async fn start_cluster_with_qwen3_5(
    agents: Vec<AgentConfig>,
    with_mmproj: bool,
) -> Result<Cluster> {
    start_cluster_with_qwen3_5_and_context_size(
        agents,
        with_mmproj,
        ModelRuntimeParameters::default().context_size,
    )
    .await
}
