use std::error::Error as _;
use std::mem::discriminant;

use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::error::ProtocolError;

use paddler_cli_tests::start_signalled_subprocess_cluster::start_signalled_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster::SubprocessCluster;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::raw_inference_socket::RawInferenceSocket;

#[tokio::test(flavor = "multi_thread")]
async fn raw_inference_socket_waiting_for_an_answer_reports_a_balancer_that_vanished() {
    let SubprocessCluster {
        balancer_signals,
        cluster,
    } = start_signalled_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams::without_agents(),
    )
    .await
    .expect("a cluster without agents must start");
    let mut raw_inference_socket =
        RawInferenceSocket::connect(cluster.balancer.addresses.inference)
            .await
            .expect("the raw socket must reach the inference socket");

    balancer_signals
        .kill()
        .expect("the balancer must be killed");

    let receive_error = raw_inference_socket
        .next_answer()
        .await
        .expect_err("a vanished balancer must not answer");

    assert_eq!(
        discriminant(&receive_error),
        discriminant(&ClusterHarnessError::InferenceSocketReceiveFailed(
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

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down after its balancer was killed");
}
