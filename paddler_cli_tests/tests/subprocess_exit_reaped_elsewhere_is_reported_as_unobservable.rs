use std::process::Stdio;

use nix::errno::Errno;
use nix::sys::wait::waitpid;
use nix::unistd::Pid;

use paddler_cli_tests::paddler_command::paddler_command;
use paddler_cli_tests::subprocess_cluster_error::SubprocessClusterError;
use paddler_cli_tests::subprocess_process::SubprocessProcess;
use paddler_cluster_harness::managed_process::ManagedProcess as _;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_exit_reaped_elsewhere_is_reported_as_unobservable() {
    let rejected_invocation = paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
        .arg("--flag-paddler-does-not-have")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("paddler must start");
    let rejected_invocation_pid = Pid::from_raw(
        rejected_invocation
            .id()
            .expect("a running invocation must have a pid")
            .cast_signed(),
    );

    waitpid(rejected_invocation_pid, None)
        .expect("the rejected invocation must exit and be reaped before the harness waits");

    assert!(matches!(
        SubprocessProcess::new(rejected_invocation)
            .exited()
            .await
            .into_result()
            .err()
            .as_ref()
            .and_then(|exit_error| exit_error.downcast_ref::<SubprocessClusterError>()),
        Some(SubprocessClusterError::ProcessExitUnobservable(exit_observation_error))
            if exit_observation_error.raw_os_error() == Some(Errno::ECHILD as i32)
    ));
}
