use std::fs::copy;
use std::fs::rename;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use tempfile::TempDir;

use paddler_test_cluster_harness::fixture_path::fixture_path;

pub struct ReplaceablePointerHead {
    pub path: PathBuf,
    directory: TempDir,
}

impl ReplaceablePointerHead {
    /// # Errors
    ///
    /// Returns an error when the temporary pointer head directory cannot be created, and when the pointer head fixture cannot be copied into it.
    pub fn holding_fixture(fixture_file_name: &str) -> Result<Self> {
        TempDir::new()
            .context("a temporary pointer head directory must be creatable")
            .and_then(|directory| {
                let path = directory.path().join("pointer_head.gguf");

                copy(fixture_path(fixture_file_name), &path)
                    .context("the pointer head fixture must be copied into place")
                    .map(|_copied_bytes| Self { path, directory })
            })
    }

    /// # Errors
    ///
    /// Returns an error when the replacing pointer head fixture cannot be staged, and when the staged pointer head cannot take the place of the pointer head.
    pub fn replace_with_fixture(&self, fixture_file_name: &str) -> Result<()> {
        let staged_path = self.directory.path().join("staged_pointer_head.gguf");

        copy(fixture_path(fixture_file_name), &staged_path)
            .context("the replacing pointer head fixture must be staged")
            .and_then(|_copied_bytes| {
                rename(&staged_path, &self.path)
                    .context("the staged pointer head must take the place of the pointer head")
            })
    }
}
