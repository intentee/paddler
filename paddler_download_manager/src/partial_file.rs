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

    #[expect(
        clippy::or_fun_call,
        reason = "Path::new is a zero-cost transmute; the lazy unwrap_or_else variant introduces an unreachable None-branch closure that cannot be covered"
    )]
    pub async fn open_for_append(&self) -> Result<File, io::Error> {
        let parent = self.partial_path.parent().unwrap_or(Path::new("."));

        fs::create_dir_all(parent).await?;

        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.partial_path)
            .await
    }

    #[expect(
        clippy::or_fun_call,
        reason = "Path::new is a zero-cost transmute; the lazy unwrap_or_else variant introduces an unreachable None-branch closure that cannot be covered"
    )]
    pub async fn truncate(&self) -> Result<(), io::Error> {
        let parent = self.partial_path.parent().unwrap_or(Path::new("."));

        fs::create_dir_all(parent).await?;

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
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test setup primitives must not fail on a healthy CI box; an unexpected error here is an environmental problem"
)]
mod tests {
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    use crate::partial_file::PartialFile;

    #[tokio::test]
    async fn current_size_returns_zero_when_missing() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        let size = partial.current_size().await.expect("current_size succeeds");

        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn current_size_returns_existing_size() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"twelve bytes")
            .await
            .expect("write partial");

        let size = partial.current_size().await.expect("current_size succeeds");

        assert_eq!(size, 12);
    }

    #[tokio::test]
    async fn open_for_append_creates_when_missing() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        let mut file = partial.open_for_append().await.expect("open succeeds");
        file.write_all(b"hello").await.expect("write succeeds");
        file.flush().await.expect("flush succeeds");

        let bytes = tokio::fs::read(&partial.partial_path)
            .await
            .expect("read back succeeds");
        assert_eq!(bytes, b"hello");
    }

    #[tokio::test]
    async fn open_for_append_appends_to_existing() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"first")
            .await
            .expect("seed partial");

        let mut file = partial.open_for_append().await.expect("open succeeds");
        file.write_all(b"-second").await.expect("write succeeds");
        file.flush().await.expect("flush succeeds");

        let bytes = tokio::fs::read(&partial.partial_path)
            .await
            .expect("read back succeeds");
        assert_eq!(bytes, b"first-second");
    }

    #[tokio::test]
    async fn truncate_resets_to_zero() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"keep me?")
            .await
            .expect("seed partial");

        partial.truncate().await.expect("truncate succeeds");

        let size = partial.current_size().await.expect("current_size succeeds");
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn finalize_renames_partial_to_final() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"complete")
            .await
            .expect("seed partial");
        let final_path = partial.final_path.clone();

        partial.finalize().await.expect("finalize succeeds");

        let exists = tokio::fs::try_exists(&final_path)
            .await
            .expect("try_exists succeeds");
        assert!(exists);
        let bytes = tokio::fs::read(&final_path)
            .await
            .expect("read final succeeds");
        assert_eq!(bytes, b"complete");
    }

    #[tokio::test]
    async fn remove_deletes_partial() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"go away")
            .await
            .expect("seed partial");
        let partial_path = partial.partial_path.clone();

        partial.remove().await.expect("remove succeeds");

        let exists = tokio::fs::try_exists(&partial_path)
            .await
            .expect("try_exists succeeds");
        assert!(!exists);
    }

    #[tokio::test]
    async fn remove_is_noop_when_missing() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));

        partial.remove().await.expect("remove is noop when missing");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn current_size_propagates_non_notfound_error() {
        let directory = TempDir::new().expect("create tempdir");
        let blocking_file = directory.path().join("blocker");
        tokio::fs::write(&blocking_file, b"a regular file")
            .await
            .expect("write blocker");
        let partial = PartialFile::new(blocking_file.join("subdir").join("model.gguf"));

        let result = partial.current_size().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn truncate_returns_io_error_when_partial_is_a_directory() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::create_dir(&partial.partial_path)
            .await
            .expect("create dir at partial path");

        let result = partial.truncate().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_for_append_returns_io_error_when_partial_is_a_directory() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::create_dir(&partial.partial_path)
            .await
            .expect("create dir at partial path");

        let result = partial.open_for_append().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn finalize_returns_io_error_when_final_is_a_non_empty_directory() {
        let directory = TempDir::new().expect("create tempdir");
        let partial = PartialFile::new(directory.path().join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"complete")
            .await
            .expect("seed partial");
        tokio::fs::create_dir(&partial.final_path)
            .await
            .expect("create dir at final path");
        tokio::fs::write(partial.final_path.join("blocker"), b"x")
            .await
            .expect("populate final dir");

        let result = partial.finalize().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn remove_propagates_non_notfound_error() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TempDir::new().expect("create tempdir");
        let locked_parent = directory.path().join("locked");
        tokio::fs::create_dir(&locked_parent)
            .await
            .expect("create locked parent");
        let partial = PartialFile::new(locked_parent.join("model.gguf"));
        tokio::fs::write(&partial.partial_path, b"go away")
            .await
            .expect("seed partial");
        let mut perms = tokio::fs::metadata(&locked_parent)
            .await
            .expect("read perms")
            .permissions();
        perms.set_mode(0o500);
        tokio::fs::set_permissions(&locked_parent, perms)
            .await
            .expect("set restrictive perms");

        let result = partial.remove().await;

        let mut restore = tokio::fs::metadata(&locked_parent)
            .await
            .expect("read perms for restore")
            .permissions();
        restore.set_mode(0o700);
        tokio::fs::set_permissions(&locked_parent, restore)
            .await
            .expect("restore perms");

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn open_for_append_fails_when_parent_blocked_by_file() {
        let directory = TempDir::new().expect("create tempdir");
        let blocker = directory.path().join("blocker");
        tokio::fs::write(&blocker, b"i am a file")
            .await
            .expect("write blocker");
        let partial = PartialFile::new(blocker.join("subdir").join("model.gguf"));

        let result = partial.open_for_append().await;

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn truncate_fails_when_parent_blocked_by_file() {
        let directory = TempDir::new().expect("create tempdir");
        let blocker = directory.path().join("blocker");
        tokio::fs::write(&blocker, b"i am a file")
            .await
            .expect("write blocker");
        let partial = PartialFile::new(blocker.join("subdir").join("model.gguf"));

        let result = partial.truncate().await;

        assert!(result.is_err());
    }
}
