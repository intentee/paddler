use std::fs::OpenOptions;
use std::os::unix::process::ExitStatusExt as _;
use std::process::ExitStatus;
use std::process::Stdio;

use paddler_cli_tests::paddler_command::paddler_command;
use paddler_test_cluster_harness::failed_run_exit_code::FAILED_RUN_EXIT_CODE;

use crate::ephemeral_balancer_arguments::ephemeral_balancer_arguments;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_exits_with_failure_when_it_cannot_announce_its_addresses() {
    assert_eq!(
        paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
            .arg("balancer")
            .args(ephemeral_balancer_arguments())
            .stdout(
                OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .expect("the full device must open for writing"),
            )
            .stderr(Stdio::null())
            .status()
            .await
            .expect("the balancer must run"),
        ExitStatus::from_raw(FAILED_RUN_EXIT_CODE << 8)
    );
}
