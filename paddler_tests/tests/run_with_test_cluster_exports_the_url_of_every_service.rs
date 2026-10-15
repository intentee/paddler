#![cfg(feature = "tests_that_use_llms")]

use std::process::Command;

#[test]
fn run_with_test_cluster_exports_the_url_of_every_service() {
    assert!(
        Command::new(env!("CARGO_BIN_EXE_run_with_test_cluster"))
            .args([
                "qwen3-0-6b",
                "--",
                "printenv",
                "PADDLER_COMPAT_OPENAI_URL",
                "PADDLER_COMPAT_TYPESAFE_URL",
                "PADDLER_INFERENCE_URL",
                "PADDLER_MANAGEMENT_URL",
            ])
            .status()
            .expect("the test cluster runner must run")
            .success()
    );
}
