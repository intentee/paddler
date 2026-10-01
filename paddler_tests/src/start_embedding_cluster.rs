use anyhow::Result;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;

use crate::start_cluster::start_cluster;

pub async fn start_embedding_cluster(
    embedding_cluster_params: EmbeddingClusterParams,
) -> Result<Cluster> {
    start_cluster(embedding_cluster_params.into_cluster_params()).await
}
