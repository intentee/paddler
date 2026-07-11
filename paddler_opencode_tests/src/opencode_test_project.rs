use std::fs;
use std::path::Path;

use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use url::Url;

use crate::opencode_test_error::OpenCodeTestError;

const MODEL_ID: &str = "paddler-opencode-test";
const MARKER_FILE_NAME: &str = "marker.txt";
const CONFIG_FILE_NAME: &str = "opencode.json";

fn provider_config(api_base_url: &Url) -> Value {
    let mut models = Map::new();

    models.insert(
        MODEL_ID.to_owned(),
        json!({ "id": MODEL_ID, "name": "Paddler OpenCode test" }),
    );

    json!({
        "$schema": "https://opencode.ai/config.json",
        "model": format!("paddler/{MODEL_ID}"),
        "provider": {
            "paddler": {
                "npm": "@ai-sdk/openai-compatible",
                "name": "Paddler",
                "options": {
                    "baseURL": api_base_url.as_str(),
                    "apiKey": "paddler"
                },
                "models": models
            }
        }
    })
}

pub struct OpenCodeTestProject {
    directory: TempDir,
    marker_contents: String,
}

impl OpenCodeTestProject {
    pub fn create(api_base_url: &Url, marker_contents: String) -> Result<Self, OpenCodeTestError> {
        let directory = tempfile::tempdir()
            .map_err(|source| OpenCodeTestError::ProjectSetupFailed { source })?;
        let config_contents = serde_json::to_string_pretty(&provider_config(api_base_url))
            .map_err(|source| OpenCodeTestError::ConfigSerializationFailed { source })?;

        fs::write(directory.path().join(CONFIG_FILE_NAME), config_contents)
            .map_err(|source| OpenCodeTestError::ProjectSetupFailed { source })?;
        fs::write(directory.path().join(MARKER_FILE_NAME), &marker_contents)
            .map_err(|source| OpenCodeTestError::ProjectSetupFailed { source })?;

        Ok(Self {
            directory,
            marker_contents,
        })
    }

    #[must_use]
    pub fn directory_path(&self) -> &Path {
        self.directory.path()
    }

    #[must_use]
    pub const fn marker_file_name(&self) -> &str {
        MARKER_FILE_NAME
    }

    #[must_use]
    pub fn marker_contents(&self) -> &str {
        &self.marker_contents
    }

    #[must_use]
    pub fn model_reference(&self) -> String {
        format!("paddler/{MODEL_ID}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_project() -> OpenCodeTestProject {
        let api_base_url = Url::parse("http://127.0.0.1:9/v1").unwrap();

        OpenCodeTestProject::create(&api_base_url, "SECRET-MARKER".to_owned()).unwrap()
    }

    #[test]
    fn config_points_at_the_compat_base_url_and_model() {
        let project = create_project();
        let config: Value = serde_json::from_str(
            &fs::read_to_string(project.directory_path().join(CONFIG_FILE_NAME)).unwrap(),
        )
        .unwrap();

        assert_eq!(
            config["provider"]["paddler"]["options"]["baseURL"],
            "http://127.0.0.1:9/v1"
        );
        assert_eq!(config["model"], "paddler/paddler-opencode-test");
        assert!(config["provider"]["paddler"]["models"][MODEL_ID].is_object());
    }

    #[test]
    fn marker_file_is_written_with_its_contents() {
        let project = create_project();

        let written =
            fs::read_to_string(project.directory_path().join(project.marker_file_name())).unwrap();

        assert_eq!(written, "SECRET-MARKER");
        assert_eq!(project.marker_contents(), "SECRET-MARKER");
    }

    #[test]
    fn model_reference_matches_the_config_model() {
        assert_eq!(
            create_project().model_reference(),
            "paddler/paddler-opencode-test"
        );
    }
}
