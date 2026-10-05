use std::io;
use std::path::Path;
use std::path::PathBuf;

use tokio::fs::File;
use tokio::fs::OpenOptions;
use tokio::fs::create_dir_all;
use tokio::fs::metadata;
use tokio::fs::remove_file;
use tokio::fs::rename;

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
        match metadata(&self.partial_path).await {
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
        rename(&self.partial_path, &self.final_path).await
    }

    pub async fn remove(&self) -> Result<(), io::Error> {
        match remove_file(&self.partial_path).await {
            Ok(()) => Ok(()),
            Err(remove_error) if remove_error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(remove_error) => Err(remove_error),
        }
    }

    async fn ensure_partial_parent_exists(&self) -> Result<(), io::Error> {
        let parent = self.partial_path.parent().unwrap_or_else(|| Path::new("."));

        create_dir_all(parent).await
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::path::PathBuf;

    use tempfile::TempDir;
    use tokio::fs::create_dir;
    use tokio::fs::create_dir_all;
    use tokio::fs::metadata;
    use tokio::fs::read;
    use tokio::fs::remove_dir_all;
    use tokio::fs::set_permissions;
    use tokio::fs::try_exists;
    use tokio::fs::write;
    use tokio::io::AsyncWriteExt;

    use crate::partial_file::PartialFile;

    #[tokio::test]
    async fn current_size_returns_zero_when_missing() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        let size = partial.current_size().await.unwrap();

        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn current_size_returns_existing_size() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        write(&partial.partial_path, b"twelve bytes").await.unwrap();

        let size = partial.current_size().await.unwrap();

        assert_eq!(size, 12);
    }

    #[tokio::test]
    async fn open_for_append_creates_when_missing() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        let mut file = partial.open_for_append().await.unwrap();
        file.write_all(b"hello").await.unwrap();
        file.flush().await.unwrap();

        let bytes = read(&partial.partial_path).await.unwrap();
        assert_eq!(bytes, b"hello");
    }

    #[tokio::test]
    async fn open_for_append_appends_to_existing() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        write(&partial.partial_path, b"first").await.unwrap();

        let mut file = partial.open_for_append().await.unwrap();
        file.write_all(b"-second").await.unwrap();
        file.flush().await.unwrap();

        let bytes = read(&partial.partial_path).await.unwrap();
        assert_eq!(bytes, b"first-second");
    }

    #[tokio::test]
    async fn truncate_resets_to_zero() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        write(&partial.partial_path, b"keep me?").await.unwrap();

        partial.truncate().await.unwrap();

        let size = partial.current_size().await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn finalize_renames_partial_to_final() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        write(&partial.partial_path, b"complete").await.unwrap();
        let final_path = partial.final_path.clone();

        partial.finalize().await.unwrap();

        let exists = try_exists(&final_path).await.unwrap();
        assert!(exists);
        let bytes = read(&final_path).await.unwrap();
        assert_eq!(bytes, b"complete");
    }

    #[tokio::test]
    async fn remove_deletes_partial() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        write(&partial.partial_path, b"go away").await.unwrap();
        let partial_path = partial.partial_path.clone();

        partial.remove().await.unwrap();

        let exists = try_exists(&partial_path).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn remove_is_noop_when_missing() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        partial.remove().await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn current_size_propagates_non_notfound_error() {
        let directory = TempDir::new().unwrap();
        let blocking_file = directory.path().join("blocker");
        write(&blocking_file, b"a regular file").await.unwrap();
        let partial = PartialFile::new(blocking_file.join("subdir").join("model.gguf"));

        let result = partial.current_size().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn truncate_returns_io_error_when_partial_is_a_directory() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        create_dir(&partial.partial_path).await.unwrap();

        let result = partial.truncate().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_for_append_returns_io_error_when_partial_is_a_directory() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        create_dir(&partial.partial_path).await.unwrap();

        let result = partial.open_for_append().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_for_append_returns_io_error_when_path_has_no_parent() {
        let partial = PartialFile::new(PathBuf::from("/"));

        let result = partial.open_for_append().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn finalize_returns_io_error_when_final_is_a_non_empty_directory() {
        let directory = TempDir::new().unwrap();
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        write(&partial.partial_path, b"complete").await.unwrap();
        create_dir(&partial.final_path).await.unwrap();
        write(partial.final_path.join("blocker"), b"x")
            .await
            .unwrap();

        let result = partial.finalize().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn remove_propagates_non_notfound_error() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TempDir::new().unwrap();
        let locked_parent = directory.path().join("locked");
        create_dir(&locked_parent).await.unwrap();
        let partial = PartialFile::new(locked_parent.join("model.gguf"));
        write(&partial.partial_path, b"go away").await.unwrap();
        let mut perms = metadata(&locked_parent).await.unwrap().permissions();
        perms.set_mode(0o500);
        set_permissions(&locked_parent, perms).await.unwrap();

        let result = partial.remove().await;

        let mut restore = metadata(&locked_parent).await.unwrap().permissions();
        restore.set_mode(0o700);
        set_permissions(&locked_parent, restore).await.unwrap();

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_for_append_fails_when_parent_blocked_by_file() {
        let directory = TempDir::new().unwrap();
        let blocker = directory.path().join("blocker");
        write(&blocker, b"i am a file").await.unwrap();
        let partial = PartialFile::new(blocker.join("subdir").join("model.gguf"));

        let result = partial.open_for_append().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn truncate_fails_when_parent_blocked_by_file() {
        let directory = TempDir::new().unwrap();
        let blocker = directory.path().join("blocker");
        write(&blocker, b"i am a file").await.unwrap();
        let partial = PartialFile::new(blocker.join("subdir").join("model.gguf"));

        let result = partial.truncate().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn finalize_returns_io_error_when_parent_was_deleted_mid_download() {
        let directory = TempDir::new().unwrap();
        let cache_subdir = directory.path().join("model-cache");
        let dest = cache_subdir.join("model.gguf");
        let partial = PartialFile::new(dest);

        create_dir_all(&cache_subdir).await.unwrap();
        let mut file = partial.open_for_append().await.unwrap();
        file.write_all(b"partial data").await.unwrap();
        file.flush().await.unwrap();
        drop(file);

        remove_dir_all(&cache_subdir).await.unwrap();

        let result = partial.finalize().await;

        assert!(result.is_err());
    }
}
