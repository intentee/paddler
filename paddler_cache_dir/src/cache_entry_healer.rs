use std::io;
use std::path::PathBuf;

use tokio::fs;

pub struct CacheEntryHealer {
    path: PathBuf,
}

impl CacheEntryHealer {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn remove_if_invalid(&self) -> Result<(), io::Error> {
        let metadata = match fs::symlink_metadata(&self.path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };

        if metadata.is_file() {
            return Ok(());
        }

        if metadata.is_dir() {
            fs::remove_dir_all(&self.path).await
        } else {
            fs::remove_file(&self.path).await
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::cache_entry_healer::CacheEntryHealer;

    #[tokio::test]
    async fn remove_if_invalid_keeps_regular_file() {
        let directory = TempDir::new().unwrap();
        let file_path = directory.path().join("model.gguf");
        tokio::fs::write(&file_path, b"real model bytes")
            .await
            .unwrap();

        CacheEntryHealer::new(file_path.clone())
            .remove_if_invalid()
            .await
            .unwrap();

        assert_eq!(
            tokio::fs::read(&file_path).await.unwrap(),
            b"real model bytes"
        );
    }

    #[tokio::test]
    async fn remove_if_invalid_removes_directory() {
        let directory = TempDir::new().unwrap();
        let directory_at_cache_path = directory.path().join("model.gguf");
        tokio::fs::create_dir(&directory_at_cache_path)
            .await
            .unwrap();
        tokio::fs::write(directory_at_cache_path.join("inner"), b"leftover")
            .await
            .unwrap();

        CacheEntryHealer::new(directory_at_cache_path.clone())
            .remove_if_invalid()
            .await
            .unwrap();

        assert!(
            !tokio::fs::try_exists(&directory_at_cache_path)
                .await
                .unwrap()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn remove_if_invalid_removes_symlink() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let symlink_at_cache_path = directory.path().join("model.gguf");
        symlink(
            directory.path().join("missing-target"),
            &symlink_at_cache_path,
        )
        .unwrap();

        CacheEntryHealer::new(symlink_at_cache_path.clone())
            .remove_if_invalid()
            .await
            .unwrap();

        assert!(
            tokio::fs::symlink_metadata(&symlink_at_cache_path)
                .await
                .is_err(),
            "the symlink itself must be removed, not followed"
        );
    }

    #[tokio::test]
    async fn remove_if_invalid_is_noop_when_absent() {
        let directory = TempDir::new().unwrap();
        let missing_path = directory.path().join("missing.gguf");

        CacheEntryHealer::new(missing_path.clone())
            .remove_if_invalid()
            .await
            .unwrap();

        assert!(!tokio::fs::try_exists(&missing_path).await.unwrap());
    }

    #[tokio::test]
    async fn remove_if_invalid_propagates_error_when_parent_is_a_file() {
        let directory = TempDir::new().unwrap();
        let file_in_place_of_parent = directory.path().join("not-a-directory");
        tokio::fs::write(&file_in_place_of_parent, b"blocker")
            .await
            .unwrap();

        let result = CacheEntryHealer::new(file_in_place_of_parent.join("child.gguf"))
            .remove_if_invalid()
            .await;

        assert!(
            result.is_err(),
            "a non-NotFound stat failure must propagate instead of being treated as healed"
        );
    }
}
