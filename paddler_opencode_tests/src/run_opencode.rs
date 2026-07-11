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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawning_a_missing_binary_reports_spawn_failure() {
        let api_base_url = url::Url::parse("http://127.0.0.1:9/v1").unwrap();
        let project = OpenCodeTestProject::create(&api_base_url, "marker".to_owned()).unwrap();

        let error = run_opencode(
            Path::new("/paddler/definitely/missing/opencode"),
            &project,
            "hello",
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();

        assert!(matches!(error, OpenCodeTestError::SpawnFailed { .. }));
    }

    #[tokio::test]
    async fn captures_stdout_and_success_of_a_finished_process() {
        let api_base_url = url::Url::parse("http://127.0.0.1:9/v1").unwrap();
        let project = OpenCodeTestProject::create(&api_base_url, "marker".to_owned()).unwrap();

        let outcome = run_opencode(
            Path::new("echo"),
            &project,
            "paddler-probe-argument",
            Duration::from_secs(30),
        )
        .await
        .unwrap();

        assert!(outcome.exit_success);
        assert!(outcome.stdout.contains("paddler-probe-argument"));
    }
}
