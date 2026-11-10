use std::net::SocketAddr;

use anyhow::Result;

use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::constants::EPHEMERAL_LOOPBACK_ADDR;

use crate::connect_subprocess_balancer::connect_subprocess_balancer;
use crate::subprocess_cluster::SubprocessCluster;

/// # Errors
///
/// Fails when the restarted balancer subprocess cannot be spawned, signalled or addressed, when it does not announce its addresses, and when the cluster cannot connect to it.
pub async fn restart_balancer(
    binary_path: &str,
    management_addr: SocketAddr,
) -> Result<SubprocessCluster> {
    connect_subprocess_balancer(
        binary_path,
        [
            "balancer".to_owned(),
            "--inference-addr".to_owned(),
            EPHEMERAL_LOOPBACK_ADDR.to_string(),
            "--management-addr".to_owned(),
            management_addr.to_string(),
        ],
        &ClusterDesiredState::KeepStored,
    )
    .await
}
