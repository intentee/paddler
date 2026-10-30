#![cfg(feature = "tests_that_use_llms")]

use std::fs::rename;
use std::fs::write;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;

pub struct SwappableModelFile {
    model_directory: TempDir,
}

impl SwappableModelFile {
    pub fn link_to(source_model_path: &Path) -> Self {
        let swappable_model_file = Self {
            model_directory: TempDir::new().expect("a temporary model directory must be creatable"),
        };

        symlink(source_model_path, swappable_model_file.model_path())
            .expect("the model must be linkable");

        swappable_model_file
    }

    pub fn agent_desired_model(&self) -> AgentDesiredModel {
        agent_desired_model_at(&self.model_path())
            .expect("the swappable model file must be at an absolute path")
    }

    pub fn model_path(&self) -> PathBuf {
        self.model_directory.path().join("model.gguf")
    }

    pub fn replace_with_an_unloadable_file(&self) {
        let unloadable_model_path = self.model_directory.path().join("unloadable.gguf");

        write(&unloadable_model_path, b"not a model")
            .expect("the unloadable file must be writable");
        rename(&unloadable_model_path, self.model_path())
            .expect("the unloadable file must replace the model");
    }
}
