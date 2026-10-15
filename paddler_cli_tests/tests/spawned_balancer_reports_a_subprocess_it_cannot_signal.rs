use paddler_cli_tests::signal_as_already_reaped::signal_as_already_reaped;
use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn spawned_balancer_reports_a_subprocess_it_cannot_signal() {
    let ephemeral_loopback_addr = EPHEMERAL_LOOPBACK_ADDR.to_string();

    assert_eq!(
        spawn_balancer_subprocess(
            env!("CARGO_BIN_EXE_paddler_cluster_node"),
            [
                "--inference-addr",
                &ephemeral_loopback_addr,
                "--management-addr",
                &ephemeral_loopback_addr,
            ],
            signal_as_already_reaped,
        )
        .await
        .err()
        .as_ref()
        .and_then(|spawn_error| spawn_error.downcast_ref::<SubprocessClusterError>())
        .map(ToString::to_string),
        Some(SubprocessClusterError::ProcessAlreadyReaped.to_string())
    );
}
