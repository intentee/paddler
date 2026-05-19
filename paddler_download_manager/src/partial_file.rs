use std::io;
use std::path::Path;
use std::path::PathBuf;

use tokio::fs;
use tokio::fs::File;
use tokio::fs::OpenOptions;

const PARTIAL_EXTENSION: &str = "partial";

pub struct PartialFile {
    pub final_path: PathBuf,
    pub partial_path: PathBuf,
}

impl PartialFile {
    #[must_use]
    pub fn new(final_path: PathBuf) -> Self {
        let partial_path = final_path.with_extension(PARTIAL_EXTENSION);

        Self {
            final_path,
            partial_path,
        }
    }

    pub async fn current_size(&self) -> Result<u64, io::Error> {
        match fs::metadata(&self.partial_path).await {
            Ok(metadata) => Ok(metadata.len()),
            Err(metadata_error) if metadata_error.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(metadata_error) => Err(metadata_error),
        }
    }

    pub async fn open_for_append(&self) -> Result<File, io::Error> {
        self.ensure_partial_parent_exists().await?;

        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.partial_path)
            .await
    }

    pub async fn truncate(&self) -> Result<(), io::Error> {
        self.ensure_partial_parent_exists().await?;

        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.partial_path)
            .await?;

        Ok(())
    }

    pub async fn finalize(&self) -> Result<(), io::Error> {
        fs::rename(&self.partial_path, &self.final_path).await
    }

    pub async fn remove(&self) -> Result<(), io::Error> {
        match fs::remove_file(&self.partial_path).await {
            Ok(()) => Ok(()),
            Err(remove_error) if remove_error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(remove_error) => Err(remove_error),
        }
    }

    async fn ensure_partial_parent_exists(&self) -> Result<(), io::Error> {
        let parent = self
            .partial_path
            .parent()
            .unwrap_or_else(|| Path::new("."));

        fs::create_dir_all(parent).await
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    use crate::partial_file::PartialFile;

    #[tokio::test]
    async fn current_size_returns_zero_when_missing() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        let size = partial.current_size().await?;

        assert_eq!(size, 0);

        Ok(())
    }

    #[tokio::test]
    async fn current_size_returns_existing_size() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"twelve bytes").await?;

        let size = partial.current_size().await?;

        assert_eq!(size, 12);

        Ok(())
    }

    #[tokio::test]
    async fn open_for_append_creates_when_missing() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        let mut file = partial.open_for_append().await?;
        file.write_all(b"hello").await?;
        file.flush().await?;

        let bytes = tokio::fs::read(&partial.partial_path).await?;
        assert_eq!(bytes, b"hello");

        Ok(())
    }

    #[tokio::test]
    async fn open_for_append_appends_to_existing() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"first").await?;

        let mut file = partial.open_for_append().await?;
        file.write_all(b"-second").await?;
        file.flush().await?;

        let bytes = tokio::fs::read(&partial.partial_path).await?;
        assert_eq!(bytes, b"first-second");

        Ok(())
    }

    #[tokio::test]
    async fn truncate_resets_to_zero() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"keep me?").await?;

        partial.truncate().await?;

        let size = partial.current_size().await?;
        assert_eq!(size, 0);

        Ok(())
    }

    #[tokio::test]
    async fn finalize_renames_partial_to_final() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"complete").await?;
        let final_path = partial.final_path.clone();

        partial.finalize().await?;

        let exists = tokio::fs::try_exists(&final_path).await?;
        assert!(exists);
        let bytes = tokio::fs::read(&final_path).await?;
        assert_eq!(bytes, b"complete");

        Ok(())
    }

    #[tokio::test]
    async fn remove_deletes_partial() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"go away").await?;
        let partial_path = partial.partial_path.clone();

        partial.remove().await?;

        let exists = tokio::fs::try_exists(&partial_path).await?;
        assert!(!exists);

        Ok(())
    }

    #[tokio::test]
    async fn remove_is_noop_when_missing() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        partial.remove().await?;

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn current_size_propagates_non_notfound_error() -> Result<()> {
        let directory = TempDir::new()?;
        let blocking_file = directory.path().join("blocker");
        tokio::fs::write(&blocking_file, b"a regular file").await?;
        let partial = PartialFile::new(blocking_file.join("subdir").join("model.gguf"));

        let result = partial.current_size().await;

        assert!(result.is_err());

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn truncate_returns_io_error_when_partial_is_a_directory() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::create_dir(&partial.partial_path).await?;

        let result = partial.truncate().await;

        assert!(result.is_err());

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_for_append_returns_io_error_when_partial_is_a_directory() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::create_dir(&partial.partial_path).await?;

        let result = partial.open_for_append().await;

        assert!(result.is_err());

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn finalize_returns_io_error_when_final_is_a_non_empty_directory() -> Result<()> {
        let directory = TempDir::new()?;
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"complete").await?;
        tokio::fs::create_dir(&partial.final_path).await?;
        tokio::fs::write(partial.final_path.join("blocker"), b"x").await?;

        let result = partial.finalize().await;

        assert!(result.is_err());

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn remove_propagates_non_notfound_error() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let directory = TempDir::new()?;
        let locked_parent = directory.path().join("locked");
        tokio::fs::create_dir(&locked_parent).await?;
        let partial = PartialFile::new(locked_parent.join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"go away").await?;
        let mut perms = tokio::fs::metadata(&locked_parent).await?.permissions();
        perms.set_mode(0o500);
        tokio::fs::set_permissions(&locked_parent, perms).await?;

        let result = partial.remove().await;

        let mut restore = tokio::fs::metadata(&locked_parent).await?.permissions();
        restore.set_mode(0o700);
        tokio::fs::set_permissions(&locked_parent, restore).await?;

        assert!(result.is_err());

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_for_append_fails_when_parent_blocked_by_file() -> Result<()> {
        let directory = TempDir::new()?;
        let blocker = directory.path().join("blocker");
        tokio::fs::write(&blocker, b"i am a file").await?;
        let partial = PartialFile::new(blocker.join("subdir").join("model.gguf"));

        let result = partial.open_for_append().await;

        assert!(result.is_err());

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn truncate_fails_when_parent_blocked_by_file() -> Result<()> {
        let directory = TempDir::new()?;
        let blocker = directory.path().join("blocker");
        tokio::fs::write(&blocker, b"i am a file").await?;
        let partial = PartialFile::new(blocker.join("subdir").join("model.gguf"));

        let result = partial.truncate().await;

        assert!(result.is_err());

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn finalize_returns_io_error_when_parent_was_deleted_mid_download() -> Result<()> {
        let directory = TempDir::new()?;
        let cache_subdir = directory.path().join("model-cache");
        let dest = cache_subdir.join("model.gguf");
        let partial = PartialFile::new(dest);

        tokio::fs::create_dir_all(&cache_subdir).await?;
        let mut file = partial.open_for_append().await?;
        file.write_all(b"partial data").await?;
        file.flush().await?;
        drop(file);

        tokio::fs::remove_dir_all(&cache_subdir).await?;

        let result = partial.finalize().await;

        assert!(result.is_err());

        Ok(())
    }
}
