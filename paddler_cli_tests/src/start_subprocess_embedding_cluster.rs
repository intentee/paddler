use anyhow::Result;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;

use crate::start_subprocess_cluster::start_subprocess_cluster;

pub async fn start_subprocess_embedding_cluster(
    binary_path: &str,
    embedding_cluster_params: EmbeddingClusterParams,
) -> Result<Cluster> {
    start_subprocess_cluster(binary_path, embedding_cluster_params.into_cluster_params()).await
}
