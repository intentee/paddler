#![cfg(feature = "tests_that_use_llms")]

use std::process::Command;

use paddler_test_cluster_harness::constants::CA_BUNDLE_WITHOUT_CERTIFICATES;
use paddler_test_cluster_harness::constants::FAILED_RUN_EXIT_CODE;

#[test]
fn run_with_test_cluster_fails_when_its_agents_cannot_set_up_their_model_downloader() {
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_run_with_test_cluster"))
            .args(["qwen3-0-6b", "--", "true"])
            .env("SSL_CERT_FILE", CA_BUNDLE_WITHOUT_CERTIFICATES)
            .env_remove("SSL_CERT_DIR")
            .status()
            .expect("the test cluster runner must run")
            .code(),
        Some(FAILED_RUN_EXIT_CODE)
    );
}
