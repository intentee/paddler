use anyhow::Result;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;

/// # Errors
///
/// Fails when the balancer subprocess cannot be spawned, signalled or addressed, when it does not announce its addresses, when the cluster cannot connect to it, and when an agent does not register.
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
