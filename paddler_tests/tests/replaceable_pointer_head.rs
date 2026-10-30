#![cfg(feature = "tests_that_use_llms")]

use std::fs::copy;
use std::fs::rename;
use std::path::PathBuf;

use tempfile::TempDir;

use paddler_test_cluster_harness::fixture_path::fixture_path;

pub struct ReplaceablePointerHead {
    pub path: PathBuf,
    directory: TempDir,
}

impl ReplaceablePointerHead {
    pub fn holding_fixture(fixture_file_name: &str) -> Self {
        let directory =
            TempDir::new().expect("a temporary pointer head directory must be creatable");
        let path = directory.path().join("pointer_head.gguf");

        copy(fixture_path(fixture_file_name), &path)
            .expect("the pointer head fixture must be copied into place");

        Self { path, directory }
    }

    pub fn replace_with_fixture(&self, fixture_file_name: &str) {
        let staged_path = self.directory.path().join("staged_pointer_head.gguf");

        copy(fixture_path(fixture_file_name), &staged_path)
            .expect("the replacing pointer head fixture must be staged");
        rename(&staged_path, &self.path)
            .expect("the staged pointer head must take the place of the pointer head");
    }
}
