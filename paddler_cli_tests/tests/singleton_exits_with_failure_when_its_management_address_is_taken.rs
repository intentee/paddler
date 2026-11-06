use std::net::TcpListener;
use std::process::Stdio;

use paddler_cli_tests::paddler_command::paddler_command;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::failed_run_exit_code::FAILED_RUN_EXIT_CODE;

#[tokio::test(flavor = "multi_thread")]
async fn singleton_exits_with_failure_when_its_management_address_is_taken() {
    let taken_listener = TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR)
        .expect("a loopback listener must bind to an ephemeral port");
    let taken_address = taken_listener
        .local_addr()
        .expect("the bound listener must report its address");

    let singleton_output = paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
        .arg("singleton")
        .arg("--inference-addr")
        .arg(EPHEMERAL_LOOPBACK_ADDR.to_string())
        .arg("--management-addr")
        .arg(taken_address.to_string())
        .arg("--slots")
        .arg("1")
        .stderr(Stdio::null())
        .output()
        .await
        .expect("the singleton must run");

    assert_eq!(singleton_output.status.code(), Some(FAILED_RUN_EXIT_CODE));
    assert_eq!(singleton_output.stdout, Vec::<u8>::new());
}
