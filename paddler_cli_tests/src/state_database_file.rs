use anyhow::Context as _;
use anyhow::Result;
use tempfile::NamedTempFile;

pub struct StateDatabaseFile {
    pub url: String,
    _file: NamedTempFile,
}

impl StateDatabaseFile {
    pub fn new() -> Result<Self> {
        let file = NamedTempFile::new().context("failed to create temp state database file")?;
        let path = file
            .path()
            .to_str()
            .context("temp state database file path is not valid UTF-8")?;
        let url = format!("file://{path}");

        Ok(Self { _file: file, url })
    }
}
