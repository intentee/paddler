use std::path::Path;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_opencode_tests::opencode_test_error::OpenCodeTestError;
use paddler_opencode_tests::opencode_test_project::OpenCodeTestProject;
use paddler_opencode_tests::run_opencode::run_opencode;
use url::Url;

const MISSING_BINARY_TIMEOUT: Duration = Duration::from_secs(5);
const STUB_BINARY_TIMEOUT: Duration = Duration::from_secs(30);

fn test_project() -> Result<OpenCodeTestProject> {
    let api_base_url = Url::parse("http://127.0.0.1:9/v1")?;

    Ok(OpenCodeTestProject::create(
        &api_base_url,
        "marker".to_owned(),
    )?)
}

#[tokio::test]
async fn spawning_a_missing_binary_reports_spawn_failure() -> Result<()> {
    let error = run_opencode(
        Path::new("/paddler/definitely/missing/opencode"),
        &test_project()?,
        "hello",
        MISSING_BINARY_TIMEOUT,
    )
    .await
    .err()
    .context("a missing binary must fail to spawn")?;

    assert!(matches!(error, OpenCodeTestError::SpawnFailed { .. }));

    Ok(())
}

#[tokio::test]
async fn captures_stdout_and_success_of_a_finished_process() -> Result<()> {
    let outcome = run_opencode(
        Path::new(env!("CARGO_BIN_EXE_opencode_stub")),
        &test_project()?,
        "paddler-probe-argument",
        STUB_BINARY_TIMEOUT,
    )
    .await?;

    assert!(outcome.exit_success);
    assert!(outcome.stdout.contains("paddler-probe-argument"));

    Ok(())
}
