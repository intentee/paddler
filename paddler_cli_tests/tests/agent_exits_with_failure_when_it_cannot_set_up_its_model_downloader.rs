use std::process::Stdio;

use paddler_cli_tests::constants::UNREACHABLE_MANAGEMENT_ADDR;
use paddler_cli_tests::paddler_command::paddler_command;
use paddler_test_cluster_harness::constants::CA_BUNDLE_WITHOUT_CERTIFICATES;
use paddler_test_cluster_harness::constants::FAILED_RUN_EXIT_CODE;

#[tokio::test(flavor = "multi_thread")]
async fn agent_exits_with_failure_when_it_cannot_set_up_its_model_downloader() {
    assert_eq!(
        paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
            .arg("agent")
            .arg("--management-addr")
            .arg(UNREACHABLE_MANAGEMENT_ADDR.to_string())
            .arg("--slots")
            .arg("1")
            .env("SSL_CERT_FILE", CA_BUNDLE_WITHOUT_CERTIFICATES)
            .env_remove("SSL_CERT_DIR")
            .stderr(Stdio::null())
            .status()
            .await
            .expect("the agent must run")
            .code(),
        Some(FAILED_RUN_EXIT_CODE)
    );
}
