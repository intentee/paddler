use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_model_card::qwen3_cluster_params::qwen3_cluster_params;

use crate::subprocess_cluster_backend::SubprocessClusterBackend;

pub async fn start_subprocess_cluster_with_qwen3(
    binary_path: &str,
    agents: Vec<AgentConfig>,
) -> Result<Cluster> {
    Cluster::start(
        &SubprocessClusterBackend::new(binary_path),
        qwen3_cluster_params(agents),
    )
    .await
}
