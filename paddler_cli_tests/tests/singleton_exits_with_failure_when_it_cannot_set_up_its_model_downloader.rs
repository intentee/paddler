use std::process::Stdio;

use paddler_cli_tests::paddler_command::paddler_command;
use paddler_test_cluster_harness::constants::CA_BUNDLE_WITHOUT_CERTIFICATES;
use paddler_test_cluster_harness::constants::EPHEMERAL_LOOPBACK_ADDR;
use paddler_test_cluster_harness::constants::FAILED_RUN_EXIT_CODE;

#[tokio::test(flavor = "multi_thread")]
async fn singleton_exits_with_failure_when_it_cannot_set_up_its_model_downloader() {
    let singleton_output = paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
        .arg("singleton")
        .arg("--inference-addr")
        .arg(EPHEMERAL_LOOPBACK_ADDR.to_string())
        .arg("--management-addr")
        .arg(EPHEMERAL_LOOPBACK_ADDR.to_string())
        .arg("--slots")
        .arg("1")
        .env("SSL_CERT_FILE", CA_BUNDLE_WITHOUT_CERTIFICATES)
        .env_remove("SSL_CERT_DIR")
        .stderr(Stdio::null())
        .output()
        .await
        .expect("the singleton must run");

    assert_eq!(singleton_output.status.code(), Some(FAILED_RUN_EXIT_CODE));
    assert_eq!(singleton_output.stdout, Vec::<u8>::new());
}
