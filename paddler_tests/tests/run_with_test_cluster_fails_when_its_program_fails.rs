#![cfg(feature = "tests_that_use_llms")]

use std::process::Command;

#[test]
fn run_with_test_cluster_fails_when_its_program_fails() {
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_run_with_test_cluster"))
            .args(["qwen3-0-6b", "--", "false"])
            .status()
            .expect("the test cluster runner must run")
            .code(),
        Some(1)
    );
}
