use std::process::Stdio;

use paddler_cli_tests::paddler_command::paddler_command;
use paddler_cluster_harness::failed_run_exit_code::failed_run_exit_code;
use paddler_file_fixture::ca_bundle_without_certificates_path::ca_bundle_without_certificates_path;

use crate::constants::UNREACHABLE_MANAGEMENT_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn agent_exits_with_failure_when_it_cannot_set_up_its_model_downloader() {
    assert_eq!(
        paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
            .arg("agent")
            .arg("--management-addr")
            .arg(UNREACHABLE_MANAGEMENT_ADDR.to_string())
            .arg("--slots")
            .arg("1")
            .env("SSL_CERT_FILE", ca_bundle_without_certificates_path())
            .env_remove("SSL_CERT_DIR")
            .stderr(Stdio::null())
            .status()
            .await
            .expect("the agent must run")
            .code(),
        Some(failed_run_exit_code())
    );
}
