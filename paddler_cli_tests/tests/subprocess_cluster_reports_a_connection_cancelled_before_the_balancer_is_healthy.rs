use std::mem::discriminant;

use tokio_util::sync::CancellationToken;

use paddler_cli_tests::subprocess_agents_cluster_connection::SubprocessAgentsClusterConnection;
use paddler_client::error::Error as ClientError;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

use crate::running_balancer_subprocess::running_balancer_subprocess;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_reports_a_connection_cancelled_before_the_balancer_is_healthy() {
    let cancelled_connection = CancellationToken::new();

    cancelled_connection.cancel();

    assert_eq!(
        SubprocessAgentsClusterConnection {
            agent_binary_path: env!("CARGO_BIN_EXE_paddler_cluster_node"),
            cancellation_token: cancelled_connection,
            desired_state: &ClusterDesiredState::Apply(Box::default()),
            running_balancer: running_balancer_subprocess().await,
        }
        .connect()
        .await
        .err()
        .as_ref()
        .and_then(|connection_error| connection_error.root_cause().downcast_ref::<ClientError>())
        .map(discriminant),
        Some(discriminant(&ClientError::RequestCancelled {
            url: String::new()
        }))
    );
}
