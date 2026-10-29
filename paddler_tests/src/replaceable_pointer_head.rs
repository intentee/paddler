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
    pub fn holding_fixture(fixture_file_name: &str) -> Result<Self> {
        TempDir::new()
            .and_then(|directory| {
                let path = directory.path().join("pointer_head.gguf");

                copy(fixture_path(fixture_file_name), &path)
                    .map(|_copied_bytes| Self { path, directory })
            })
            .context("the pointer head fixture must be copied into a temporary directory")
    }

    pub fn replace_with_fixture(&self, fixture_file_name: &str) -> Result<()> {
        let staged_path = self.directory.path().join("staged_pointer_head.gguf");

        copy(fixture_path(fixture_file_name), &staged_path)
            .and_then(|_copied_bytes| rename(&staged_path, &self.path))
            .context("the replacing pointer head fixture must take the place of the pointer head")
    }
}
