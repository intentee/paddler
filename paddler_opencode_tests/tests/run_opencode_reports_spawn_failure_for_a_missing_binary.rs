use std::path::Path;

use paddler_opencode_tests::opencode_test_error::OpenCodeTestError;
use paddler_opencode_tests::run_opencode::run_opencode;

use crate::unreachable_api_opencode_test_project::unreachable_api_opencode_test_project;

#[tokio::test]
async fn run_opencode_reports_spawn_failure_for_a_missing_binary() {
    let run_error = run_opencode(
        Path::new("/paddler/definitely/missing/opencode"),
        &unreachable_api_opencode_test_project()
            .expect("an OpenCode test project must be creatable"),
        "hello",
    )
    .await
    .expect_err("a missing binary must fail to spawn");

    assert!(matches!(run_error, OpenCodeTestError::SpawnFailed { .. }));
}
