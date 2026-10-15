use std::error::Error as _;
use std::mem::discriminant;

use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::error::ProtocolError;

use paddler_cli_tests::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster::SubprocessCluster;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::untrusted_agent_socket_client::UntrustedAgentSocketClient;

#[tokio::test(flavor = "multi_thread")]
async fn untrusted_agent_socket_reports_a_balancer_that_vanished() {
    let SubprocessCluster {
        balancer_signals,
        cluster,
    } = start_signalled_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("a cluster without agents must start");
    let mut agent_socket = UntrustedAgentSocketClient::connect(
        cluster.balancer.addresses.management,
        "agent-of-a-vanished-balancer",
    )
    .await
    .expect("the agent socket must reach the balancer");

    balancer_signals
        .kill()
        .expect("the balancer must be killed");

    let receive_error = agent_socket
        .next_close_frame()
        .await
        .expect_err("a vanished balancer must not send a close frame");

    assert_eq!(
        receive_error.to_string(),
        ClusterHarnessError::AgentSocketReceiveFailed(WebSocketError::ConnectionClosed).to_string()
    );
    assert_eq!(
        receive_error
            .source()
            .and_then(|source| source.downcast_ref::<WebSocketError>())
            .map(ToString::to_string),
        Some(WebSocketError::Protocol(ProtocolError::ResetWithoutClosingHandshake).to_string())
    );
    assert_eq!(
        agent_socket
            .next_close_frame()
            .await
            .err()
            .as_ref()
            .map(discriminant),
        Some(discriminant(
            &ClusterHarnessError::AgentSocketEndedWithoutClosing
        ))
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down after its balancer was killed");
}
