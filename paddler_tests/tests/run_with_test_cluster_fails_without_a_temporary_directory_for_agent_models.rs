#![cfg(feature = "tests_that_use_llms")]

use std::process::Command;

const ANYHOW_MAIN_ERROR_EXIT_CODE: i32 = 1;

#[test]
fn run_with_test_cluster_fails_without_a_temporary_directory_for_agent_models() {
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_run_with_test_cluster"))
            .args(["qwen3-0-6b", "--", "true"])
            .env("TMPDIR", "/nonexistent-temporary-directory")
            .status()
            .expect("the test cluster runner must run")
            .code(),
        Some(ANYHOW_MAIN_ERROR_EXIT_CODE)
    );
}
