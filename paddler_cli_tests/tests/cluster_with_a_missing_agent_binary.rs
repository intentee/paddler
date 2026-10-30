use tokio_util::sync::CancellationToken;

use paddler_cli_tests::missing_binary_path::MISSING_BINARY_PATH;
use paddler_cli_tests::subprocess_agents_cluster_connection::SubprocessAgentsClusterConnection;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

use crate::running_balancer_subprocess::running_balancer_subprocess;

pub async fn cluster_with_a_missing_agent_binary() -> Cluster {
    SubprocessAgentsClusterConnection {
        agent_binary_path: MISSING_BINARY_PATH,
        cancellation_token: CancellationToken::new(),
        desired_state: &ClusterDesiredState::Apply(Box::default()),
        running_balancer: running_balancer_subprocess().await,
    }
    .connect()
    .await
    .expect("the cluster must connect to the balancer")
}
