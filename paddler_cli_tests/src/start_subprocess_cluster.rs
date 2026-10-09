use anyhow::Result;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;

pub async fn start_subprocess_cluster(
    binary_path: &str,
    cluster_params: ClusterParams,
) -> Result<Cluster> {
    Ok(
        start_signalled_subprocess_cluster(binary_path, cluster_params)
            .await?
            .cluster,
    )
}
