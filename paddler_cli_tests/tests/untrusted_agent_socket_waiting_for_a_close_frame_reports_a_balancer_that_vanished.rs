use std::error::Error as _;
use std::mem::discriminant;

use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::error::ProtocolError;

use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;

use crate::agent_socket_of_a_vanished_balancer::AgentSocketOfAVanishedBalancer;

#[tokio::test(flavor = "multi_thread")]
async fn untrusted_agent_socket_waiting_for_a_close_frame_reports_a_balancer_that_vanished() {
    let mut agent_socket_of_a_vanished_balancer = AgentSocketOfAVanishedBalancer::connect().await;

    let receive_error = agent_socket_of_a_vanished_balancer
        .agent_socket
        .next_close_frame()
        .await
        .expect_err("a vanished balancer must not send a close frame");

    assert_eq!(
        discriminant(&receive_error),
        discriminant(&ClusterHarnessError::AgentSocketReceiveFailed(
            WebSocketError::ConnectionClosed
        ))
    );
    assert_eq!(
        receive_error
            .source()
            .and_then(|websocket_error| websocket_error.source())
            .and_then(|protocol_error| protocol_error.downcast_ref::<ProtocolError>()),
        Some(&ProtocolError::ResetWithoutClosingHandshake)
    );
    assert_eq!(
        agent_socket_of_a_vanished_balancer
            .agent_socket
            .next_close_frame()
            .await
            .err()
            .as_ref()
            .map(discriminant),
        Some(discriminant(
            &ClusterHarnessError::AgentSocketEndedWithoutClosing
        ))
    );

    agent_socket_of_a_vanished_balancer.shut_down().await;
}
