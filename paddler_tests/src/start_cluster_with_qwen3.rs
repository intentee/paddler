use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_model_card::qwen3_cluster_params::qwen3_cluster_params;

use crate::in_process_cluster_backend::InProcessClusterBackend;

pub async fn start_cluster_with_qwen3(agents: Vec<AgentConfig>) -> Result<Cluster> {
    Cluster::start(
        &InProcessClusterBackend::default(),
        qwen3_cluster_params(agents),
    )
    .await
}
