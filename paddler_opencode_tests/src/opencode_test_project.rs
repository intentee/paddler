use std::fs;
use std::path::Path;
use std::process::Output;
use std::process::Stdio;

use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use tempfile::tempdir;
use tokio::process::Command;
use url::Url;

use crate::opencode_run_outcome::OpenCodeRunOutcome;
use crate::opencode_test_error::OpenCodeTestError;

const MODEL_ID: &str = "paddler-opencode-test";
const MARKER_FILE_NAME: &str = "marker.txt";
const CONFIG_FILE_NAME: &str = "opencode.json";
const PROVIDER_ID: &str = "paddler";
const UNREACHABLE_API_BASE_URL: &str = "http://127.0.0.1:1/v1";
const UNREACHABLE_API_MARKER_CONTENTS: &str = "marker";

fn provider_config(api_base_url: &str) -> Value {
    let mut models = Map::new();

    models.insert(
        MODEL_ID.to_owned(),
        json!({ "modelID": MODEL_ID, "name": "Paddler OpenCode test" }),
    );

    json!({
        "$schema": "https://opencode.ai/config.json",
        "model": OpenCodeTestProject::model_reference(),
        "providers": {
            PROVIDER_ID: {
                "name": "Paddler",
                "package": "@opencode/ai/providers/openai-compatible",
                "settings": {
                    "apiKey": "paddler",
                    "baseURL": api_base_url
                },
                "models": models
            }
        }
    })
}

pub struct OpenCodeTestProject {
    directory: TempDir,
    home: TempDir,
    marker_contents: String,
}

impl OpenCodeTestProject {
    /// # Errors
    ///
    /// Returns [`OpenCodeTestError::ProjectSetupFailed`] when the project or home directory cannot be created, or the configuration or marker file cannot be written.
    pub fn create(api_base_url: &Url, marker_contents: String) -> Result<Self, OpenCodeTestError> {
        Self::create_for_api_base_url(api_base_url.as_str(), marker_contents)
    }

    /// # Errors
    ///
    /// Returns [`OpenCodeTestError::ProjectSetupFailed`] when the project or home directory cannot be created, or the configuration or marker file cannot be written.
    pub fn with_an_unreachable_api() -> Result<Self, OpenCodeTestError> {
        Self::create_for_api_base_url(
            UNREACHABLE_API_BASE_URL,
            UNREACHABLE_API_MARKER_CONTENTS.to_owned(),
        )
    }

    fn create_for_api_base_url(
        api_base_url: &str,
        marker_contents: String,
    ) -> Result<Self, OpenCodeTestError> {
        tempdir()
            .and_then(|directory| {
                fs::write(
                    directory.path().join(CONFIG_FILE_NAME),
                    format!("{:#}", provider_config(api_base_url)),
                )
                .and_then(|()| fs::write(directory.path().join(MARKER_FILE_NAME), &marker_contents))
                .map(|()| directory)
            })
            .and_then(|directory| {
                tempdir().map(|home| Self {
                    directory,
                    home,
                    marker_contents,
                })
            })
            .map_err(OpenCodeTestError::ProjectSetupFailed)
    }

    #[must_use]
    pub fn directory_path(&self) -> &Path {
        self.directory.path()
    }

    #[must_use]
    pub fn home_path(&self) -> &Path {
        self.home.path()
    }

    #[must_use]
    pub fn marker_contents(&self) -> &str {
        &self.marker_contents
    }

    #[must_use]
    pub const fn marker_file_name() -> &'static str {
        MARKER_FILE_NAME
    }

    #[must_use]
    pub fn model_reference() -> String {
        format!("{PROVIDER_ID}/{MODEL_ID}")
    }

    /// # Errors
    ///
    /// Returns [`OpenCodeTestError::SpawnFailed`] when the binary cannot be spawned, and [`OpenCodeTestError::ProcessWaitFailed`] when its output cannot be collected after it finishes.
    pub async fn run_opencode(
        &self,
        binary_path: &Path,
        prompt: &str,
    ) -> Result<OpenCodeRunOutcome, OpenCodeTestError> {
        let mut command = Command::new(binary_path);
        let home = self.home_path();

        command
            .arg("run")
            .arg(prompt)
            .arg("--standalone")
            .arg("--auto")
            .arg("--model")
            .arg(Self::model_reference())
            .current_dir(self.directory_path())
            .env("PWD", self.directory_path())
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
            .map(
                |Output {
                     status,
                     stderr,
                     stdout,
                 }| OpenCodeRunOutcome {
                    exit_success: status.success(),
                    stderr: String::from_utf8_lossy(&stderr).into_owned(),
                    stdout: String::from_utf8_lossy(&stdout).into_owned(),
                },
            )
            .map_err(OpenCodeTestError::ProcessWaitFailed)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use serde_json::from_str;
    use serde_json::json;
    use url::Url;

    use super::CONFIG_FILE_NAME;
    use super::MODEL_ID;
    use super::OpenCodeTestProject;

    fn create_project() -> OpenCodeTestProject {
        let api_base_url = Url::parse("http://127.0.0.1:9/v1").unwrap();

        OpenCodeTestProject::create(&api_base_url, "SECRET-MARKER".to_owned()).unwrap()
    }

    #[test]
    fn config_points_at_the_compat_base_url_and_model() {
        let project = create_project();
        let config: Value =
            from_str(&fs::read_to_string(project.directory_path().join(CONFIG_FILE_NAME)).unwrap())
                .unwrap();

        assert_eq!(
            config,
            json!({
                "$schema": "https://opencode.ai/config.json",
                "model": "paddler/paddler-opencode-test",
                "providers": {
                    "paddler": {
                        "name": "Paddler",
                        "package": "@opencode/ai/providers/openai-compatible",
                        "settings": {
                            "apiKey": "paddler",
                            "baseURL": "http://127.0.0.1:9/v1"
                        },
                        "models": {
                            MODEL_ID: { "modelID": MODEL_ID, "name": "Paddler OpenCode test" }
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn marker_file_is_written_with_its_contents() {
        let project = create_project();

        let written = fs::read_to_string(
            project
                .directory_path()
                .join(OpenCodeTestProject::marker_file_name()),
        )
        .unwrap();

        assert_eq!(written, "SECRET-MARKER");
        assert_eq!(project.marker_contents(), "SECRET-MARKER");
    }
}
