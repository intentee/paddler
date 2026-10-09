use std::process::Stdio;

use paddler_cli_tests::clap_usage_error_exit_code::CLAP_USAGE_ERROR_EXIT_CODE;
use paddler_cli_tests::paddler_command::paddler_command;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_cli_tests::subprocess_process::SubprocessProcess;
use paddler_test_cluster_harness::managed_process::ManagedProcess as _;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_shutdown_reports_a_failed_exit() {
    let mut rejected_invocation = paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
        .arg("--flag-paddler-does-not-have")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("paddler must start");
    let _rejected_invocation_status = rejected_invocation
        .wait()
        .await
        .expect("the rejected invocation must exit");

    let shutdown_error = Box::new(SubprocessProcess::new(rejected_invocation))
        .shutdown()
        .await
        .expect_err("a failed exit must be reported at shutdown");

    assert!(matches!(
        shutdown_error.downcast_ref::<SubprocessClusterError>(),
        Some(SubprocessClusterError::ProcessExitedWithFailure { exit_status })
            if exit_status.code() == Some(CLAP_USAGE_ERROR_EXIT_CODE)
    ));
}
