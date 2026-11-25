use std::process::Stdio;

use paddler_cli_tests::paddler_command::paddler_command;
use paddler_cluster_harness::ephemeral_loopback_addr::ephemeral_loopback_addr;
use paddler_cluster_harness::failed_run_exit_code::failed_run_exit_code;
use paddler_file_fixture::ca_bundle_without_certificates_path::ca_bundle_without_certificates_path;

#[tokio::test(flavor = "multi_thread")]
async fn singleton_exits_with_failure_when_it_cannot_set_up_its_model_downloader() {
    let singleton_output = paddler_command(env!("CARGO_BIN_EXE_paddler_cluster_node"))
        .arg("singleton")
        .arg("--inference-addr")
        .arg(ephemeral_loopback_addr().to_string())
        .arg("--management-addr")
        .arg(ephemeral_loopback_addr().to_string())
        .arg("--slots")
        .arg("1")
        .env("SSL_CERT_FILE", ca_bundle_without_certificates_path())
        .env_remove("SSL_CERT_DIR")
        .stderr(Stdio::null())
        .output()
        .await
        .expect("the singleton must run");

    assert_eq!(singleton_output.status.code(), Some(failed_run_exit_code()));
    assert_eq!(singleton_output.stdout, Vec::<u8>::new());
}
