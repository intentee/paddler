use std::error::Error as _;
use std::io;
use std::path::Path;

use paddler_opencode_tests::opencode_test_error::OpenCodeTestError;
use paddler_opencode_tests::run_opencode::run_opencode;

use crate::unreachable_api_opencode_test_project::unreachable_api_opencode_test_project;

#[tokio::test]
async fn run_opencode_reports_spawn_failure_for_a_missing_binary() {
    let missing_binary = Path::new("/paddler/definitely/missing/opencode");
    let run_error = run_opencode(
        missing_binary,
        &unreachable_api_opencode_test_project(),
        "hello",
    )
    .await
    .expect_err("a missing binary must fail to spawn");

    assert_eq!(
        run_error.to_string(),
        OpenCodeTestError::SpawnFailed {
            path: missing_binary.to_path_buf(),
            source: io::Error::from(io::ErrorKind::NotFound),
        }
        .to_string()
    );
    assert_eq!(
        run_error
            .source()
            .and_then(|source| source.downcast_ref::<io::Error>())
            .map(io::Error::kind),
        Some(io::ErrorKind::NotFound)
    );
}
