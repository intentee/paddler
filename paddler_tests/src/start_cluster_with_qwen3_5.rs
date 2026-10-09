use anyhow::Result;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;

use crate::start_cluster_with_qwen3_5_and_context_size::start_cluster_with_qwen3_5_and_context_size;

pub async fn start_cluster_with_qwen3_5(
    agents: Vec<AgentConfig>,
    with_mmproj: bool,
) -> Result<Cluster> {
    start_cluster_with_qwen3_5_and_context_size(
        agents,
        with_mmproj,
        ModelRuntimeParameters::default().context_size.get(),
    )
    .await
}
