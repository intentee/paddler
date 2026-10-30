use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::opencode_run_outcome::OpenCodeRunOutcome;
use crate::opencode_test_error::OpenCodeTestError;
use crate::opencode_test_project::OpenCodeTestProject;

pub async fn run_opencode(
    binary_path: &Path,
    project: &OpenCodeTestProject,
    prompt: &str,
) -> Result<OpenCodeRunOutcome, OpenCodeTestError> {
    let mut command = Command::new(binary_path);
    let home = project.home_path();

    command
        .arg("run")
        .arg(prompt)
        .arg("--standalone")
        .arg("--auto")
        .arg("--model")
        .arg(project.model_reference())
        .current_dir(project.directory_path())
        .env("PWD", project.directory_path())
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local").join("share"))
        .env("XDG_STATE_HOME", home.join(".local").join("state"))
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = command
        .spawn()
        .map_err(|source| OpenCodeTestError::SpawnFailed {
            path: binary_path.to_path_buf(),
            source,
        })?;

    child
        .wait_with_output()
        .await
        .map(|output| OpenCodeRunOutcome {
            exit_success: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        })
        .map_err(OpenCodeTestError::ProcessWaitFailed)
}
