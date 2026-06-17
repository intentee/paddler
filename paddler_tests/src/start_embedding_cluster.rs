use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_model_card::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;

use crate::in_process_cluster_backend::InProcessClusterBackend;

pub async fn start_embedding_cluster(params: Qwen3EmbeddingClusterParams) -> Result<Cluster> {
    let backend = InProcessClusterBackend::default().with_service_config(params.service_config());

    Cluster::start(&backend, params.into_cluster_params()).await
}
