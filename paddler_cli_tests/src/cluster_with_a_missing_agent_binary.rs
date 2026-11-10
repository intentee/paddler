use anyhow::Result;
use futures_util::TryFutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

use crate::constants::MISSING_BINARY_PATH;
use crate::running_balancer_subprocess::running_balancer_subprocess;
use crate::subprocess_agents_cluster_connection::SubprocessAgentsClusterConnection;

/// # Errors
///
/// Fails when the balancer subprocess does not start and announce its addresses, and when the cluster cannot connect to it.
pub async fn cluster_with_a_missing_agent_binary(binary_path: &str) -> Result<Cluster> {
    let desired_state = ClusterDesiredState::Apply(Box::default());

    running_balancer_subprocess(binary_path)
        .and_then(|running_balancer| {
            SubprocessAgentsClusterConnection {
                agent_binary_path: MISSING_BINARY_PATH,
                cancellation_token: CancellationToken::new(),
                desired_state: &desired_state,
                running_balancer,
            }
            .connect()
        })
        .await
}
