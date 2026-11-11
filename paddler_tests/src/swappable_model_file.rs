use std::fs::rename;
use std::fs::write;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use tempfile::TempDir;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;

pub struct SwappableModelFile {
    model_directory: TempDir,
}

impl SwappableModelFile {
    /// # Errors
    ///
    /// Returns an error when the temporary model directory cannot be created, and when the model cannot be linked into it.
    pub fn link_to(source_model_path: &Path) -> Result<Self> {
        TempDir::new()
            .context("a temporary model directory must be creatable")
            .map(|model_directory| Self { model_directory })
            .and_then(|swappable_model_file| {
                symlink(source_model_path, swappable_model_file.model_path())
                    .context("the model must be linkable")
                    .map(|()| swappable_model_file)
            })
    }

    /// # Errors
    ///
    /// Returns [`ModelSourceError::LocalModelPathNotAbsolute`] when the model directory lies at a relative path, and [`ModelSourceError::LocalModelPathClimbsToParentDirectory`] when its path climbs to a parent directory.
    pub fn agent_desired_model(&self) -> Result<AgentDesiredModel, ModelSourceError> {
        agent_desired_model_at(&self.model_path())
    }

    #[must_use]
    pub fn model_path(&self) -> PathBuf {
        self.model_directory.path().join("model.gguf")
    }

    /// # Errors
    ///
    /// Returns an error when the unloadable file cannot be written, and when it cannot replace the model.
    pub fn replace_with_an_unloadable_file(&self) -> Result<()> {
        let unloadable_model_path = self.model_directory.path().join("unloadable.gguf");

        write(&unloadable_model_path, b"not a model")
            .context("the unloadable file must be writable")
            .and_then(|()| {
                rename(&unloadable_model_path, self.model_path())
                    .context("the unloadable file must replace the model")
            })
    }
}
