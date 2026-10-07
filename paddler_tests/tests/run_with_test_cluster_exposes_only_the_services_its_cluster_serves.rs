#![cfg(feature = "tests_that_use_llms")]

use std::process::Command;

#[test]
fn run_with_test_cluster_exposes_only_the_services_its_cluster_serves() {
    assert_eq!(
        [
            ["kev-0-8b", "PADDLER_COMPAT_TYPESAFE_URL"],
            ["qwen3-0-6b", "PADDLER_COMPAT_OPENAI_URL"],
            ["kev-0-8b", "PADDLER_COMPAT_OPENAI_URL"],
        ]
        .map(|[preset, variable]| {
            Command::new(env!("CARGO_BIN_EXE_run_with_test_cluster"))
                .args([preset, "--", "printenv", variable])
                .status()
                .expect("the test cluster runner must run")
                .success()
        }),
        [true, true, false]
    );
}
