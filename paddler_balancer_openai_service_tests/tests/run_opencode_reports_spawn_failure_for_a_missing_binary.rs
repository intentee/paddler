use std::error::Error as _;
use std::io;
use std::path::Path;

use paddler_opencode_tests::opencode_test_error::OpenCodeTestError;
use paddler_opencode_tests::opencode_test_project::OpenCodeTestProject;

#[tokio::test]
async fn run_opencode_reports_spawn_failure_for_a_missing_binary() {
    let missing_binary = Path::new("/paddler/definitely/missing/opencode");
    let run_error = OpenCodeTestProject::with_an_unreachable_api()
        .expect("an OpenCode test project must be creatable")
        .run_opencode(missing_binary, "hello")
        .await
        .expect_err("a missing binary must fail to spawn");

    assert!(matches!(
        &run_error,
        OpenCodeTestError::SpawnFailed { path, .. } if path == missing_binary
    ));
    assert_eq!(
        run_error
            .source()
            .and_then(|source| source.downcast_ref::<io::Error>())
            .map(io::Error::kind),
        Some(io::ErrorKind::NotFound)
    );
}
