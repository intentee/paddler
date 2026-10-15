use std::net::SocketAddr;

use anyhow::Result;

use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::connect_subprocess_balancer::connect_subprocess_balancer;
use crate::subprocess_cluster::SubprocessCluster;

pub async fn restart_balancer(
    binary_path: &str,
    management_addr: SocketAddr,
) -> Result<SubprocessCluster> {
    connect_subprocess_balancer(
        binary_path,
        [
            "--inference-addr".to_owned(),
            EPHEMERAL_LOOPBACK_ADDR.to_string(),
            "--management-addr".to_owned(),
            management_addr.to_string(),
        ],
        &ClusterDesiredState::KeepStored,
    )
    .await
}
