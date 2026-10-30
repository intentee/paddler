use std::mem::discriminant;

use serde_json::Value;
use serde_json::from_str;

use paddler_cli_tests::signalled_subprocess::SignalledSubprocess;
use paddler_cli_tests::spawn_balancer_subprocess::spawn_balancer_subprocess;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;

#[tokio::test(flavor = "multi_thread")]
async fn spawned_balancer_reports_an_announcement_that_is_not_its_addresses() {
    let spawn_error = spawn_balancer_subprocess(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ["--help"],
        SignalledSubprocess::new,
    )
    .await
    .err()
    .expect("a balancer that prints its help must not be taken for an announcement");

    let help_text_error =
        from_str::<Value>("help text").expect_err("help text must not be valid JSON");

    assert_eq!(
        spawn_error
            .downcast_ref::<SubprocessClusterError>()
            .map(discriminant),
        Some(discriminant(&SubprocessClusterError::AnnouncementInvalid {
            announcement: String::new(),
            source: help_text_error,
        }))
    );
}
