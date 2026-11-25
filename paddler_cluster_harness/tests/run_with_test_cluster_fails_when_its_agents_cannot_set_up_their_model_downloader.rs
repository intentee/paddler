#![cfg(feature = "tests_that_use_llms")]

use std::process::Command;

use paddler_cluster_harness::failed_run_exit_code::failed_run_exit_code;
use paddler_file_fixture::ca_bundle_without_certificates_path::ca_bundle_without_certificates_path;

#[test]
fn run_with_test_cluster_fails_when_its_agents_cannot_set_up_their_model_downloader() {
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_run_with_test_cluster"))
            .args(["qwen3-0-6b", "--", "true"])
            .env("SSL_CERT_FILE", ca_bundle_without_certificates_path())
            .env_remove("SSL_CERT_DIR")
            .status()
            .expect("the test cluster runner must run")
            .code(),
        Some(failed_run_exit_code())
    );
}
