use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;

#[tokio::test(flavor = "multi_thread")]
async fn spawned_balancer_reports_an_announcement_that_is_not_its_addresses() {
    let spawn_error =
        spawn_balancer_subprocess(env!("CARGO_BIN_EXE_paddler_cluster_node"), ["--help"])
            .await
            .err()
            .expect("a balancer that prints its help must not be taken for an announcement");

    assert!(matches!(
        spawn_error.downcast_ref::<SubprocessClusterError>(),
        Some(SubprocessClusterError::AnnouncementInvalid { .. })
    ));
}
