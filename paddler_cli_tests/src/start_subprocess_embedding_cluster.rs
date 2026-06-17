use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_model_card::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;

use crate::subprocess_cluster_backend::SubprocessClusterBackend;

pub async fn start_subprocess_embedding_cluster(
    binary_path: &str,
    params: Qwen3EmbeddingClusterParams,
) -> Result<Cluster> {
    let backend =
        SubprocessClusterBackend::new(binary_path).with_service_config(params.service_config());

    Cluster::start(&backend, params.into_cluster_params()).await
}
