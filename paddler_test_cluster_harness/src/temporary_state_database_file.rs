use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use tempfile::TempDir;

pub struct TemporaryStateDatabaseFile {
    pub path: PathBuf,
    pub url: String,
    _directory_guard: TempDir,
}

impl TemporaryStateDatabaseFile {
    pub fn new() -> Result<Self> {
        TempDir::new()
            .context("failed to create temp state database directory")
            .and_then(Self::in_directory)
    }

    fn in_directory(directory: TempDir) -> Result<Self> {
        let path = directory.path().join("state.json");

        path.to_str()
            .map(|utf8_path| format!("file://{utf8_path}"))
            .context("temp state database file path is not valid UTF-8")
            .map(|url| Self {
                path,
                url,
                _directory_guard: directory,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::TemporaryStateDatabaseFile;

    #[test]
    fn names_its_url_after_its_file_path() {
        let database = TemporaryStateDatabaseFile::new().unwrap();

        assert_eq!(database.url, format!("file://{}", database.path.display()));
    }
}
