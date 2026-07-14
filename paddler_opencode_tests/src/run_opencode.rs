use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

use crate::opencode_run_outcome::OpenCodeRunOutcome;
use crate::opencode_test_error::OpenCodeTestError;
use crate::opencode_test_project::OpenCodeTestProject;

pub async fn run_opencode(
    binary_path: &Path,
    project: &OpenCodeTestProject,
    prompt: &str,
    timeout: Duration,
) -> Result<OpenCodeRunOutcome, OpenCodeTestError> {
    let mut command = Command::new(binary_path);

    command
        .arg("run")
        .arg(prompt)
        .arg("--dir")
        .arg(project.directory_path())
        .arg("--pure")
        .arg("--auto")
        .arg("--model")
        .arg(project.model_reference())
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = command
        .spawn()
        .map_err(|source| OpenCodeTestError::SpawnFailed {
            path: binary_path.to_path_buf(),
            source,
        })?;

    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| OpenCodeTestError::TimedOut {
            seconds: timeout.as_secs(),
        })?
        .map_err(|source| OpenCodeTestError::ProcessWaitFailed { source })?;

    Ok(OpenCodeRunOutcome {
        exit_success: output.status.success(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
    })
}
