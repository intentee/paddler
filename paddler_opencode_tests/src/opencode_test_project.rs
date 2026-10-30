use std::fs;
use std::path::Path;

use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use tempfile::tempdir;
use url::Url;

use crate::opencode_test_error::OpenCodeTestError;

const MODEL_ID: &str = "paddler-opencode-test";
const MARKER_FILE_NAME: &str = "marker.txt";
const CONFIG_FILE_NAME: &str = "opencode.json";

fn provider_config(api_base_url: &Url) -> Value {
    let mut models = Map::new();

    models.insert(
        MODEL_ID.to_owned(),
        json!({ "modelID": MODEL_ID, "name": "Paddler OpenCode test" }),
    );

    json!({
        "$schema": "https://opencode.ai/config.json",
        "model": format!("paddler/{MODEL_ID}"),
        "providers": {
            "paddler": {
                "name": "Paddler",
                "package": "@opencode/ai/providers/openai-compatible",
                "settings": {
                    "apiKey": "paddler",
                    "baseURL": api_base_url.as_str()
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
    pub fn create(api_base_url: &Url, marker_contents: String) -> Result<Self, OpenCodeTestError> {
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
