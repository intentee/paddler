use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use tempfile::TempDir;

pub struct StateDatabaseFile {
    pub path: PathBuf,
    pub url: String,
    _directory_guard: TempDir,
}

impl StateDatabaseFile {
    pub fn new() -> Result<Self> {
        let directory = TempDir::new().context("failed to create temp state database directory")?;
        let path = directory.path().join("state.json");
        let url = format!(
            "file://{}",
            path.to_str()
                .context("temp state database file path is not valid UTF-8")?
        );

        Ok(Self {
            path,
            url,
            _directory_guard: directory,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::StateDatabaseFile;

    #[test]
    fn names_its_url_after_its_file_path() {
        let database = StateDatabaseFile::new().unwrap();

        assert_eq!(database.url, format!("file://{}", database.path.display()));
    }
}
