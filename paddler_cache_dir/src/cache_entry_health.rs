use std::fs::Metadata;
use std::io;
use std::path::PathBuf;

use tokio::fs;

use crate::cache_entry_state::CacheEntryState;

fn is_valid_cache_metadata(metadata: &Metadata) -> bool {
    metadata.is_file()
}

pub struct CacheEntryHealth {
    path: PathBuf,
}

impl CacheEntryHealth {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    async fn lookup(&self) -> Result<Option<Metadata>, io::Error> {
        match fs::symlink_metadata(&self.path).await {
            Ok(metadata) => Ok(Some(metadata)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn is_cached(&self) -> Result<bool, io::Error> {
        Ok(self
            .lookup()
            .await?
            .is_some_and(|metadata| is_valid_cache_metadata(&metadata)))
    }

    pub async fn heal(&self) -> Result<CacheEntryState, io::Error> {
        loop {
            let Some(metadata) = self.lookup().await? else {
                return Ok(CacheEntryState::Vacant);
            };

            if is_valid_cache_metadata(&metadata) {
                return Ok(CacheEntryState::Cached);
            }

            let removal = if metadata.is_dir() {
                fs::remove_dir_all(&self.path).await
            } else {
                fs::remove_file(&self.path).await
            };

            removal?;
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::cache_entry_health::CacheEntryHealth;
    use crate::cache_entry_state::CacheEntryState;

    #[tokio::test]
    async fn heal_returns_cached_for_regular_file() {
        let directory = TempDir::new().unwrap();
        let file_path = directory.path().join("model.gguf");
        tokio::fs::write(&file_path, b"model bytes").await.unwrap();

        assert_eq!(
            CacheEntryHealth::new(file_path).heal().await.unwrap(),
            CacheEntryState::Cached
        );
    }

    #[tokio::test]
    async fn heal_removes_directory_and_returns_vacant() {
        let directory = TempDir::new().unwrap();
        let directory_at_cache_path = directory.path().join("model.gguf");
        tokio::fs::create_dir(&directory_at_cache_path)
            .await
            .unwrap();
        tokio::fs::write(directory_at_cache_path.join("inner"), b"leftover")
            .await
            .unwrap();

        assert_eq!(
            CacheEntryHealth::new(directory_at_cache_path.clone())
                .heal()
                .await
                .unwrap(),
            CacheEntryState::Vacant
        );
        assert!(
            !tokio::fs::try_exists(&directory_at_cache_path)
                .await
                .unwrap()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn heal_removes_symlink_and_returns_vacant() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let real_file = directory.path().join("real.gguf");
        tokio::fs::write(&real_file, b"real model bytes")
            .await
            .unwrap();
        let symlink_at_cache_path = directory.path().join("model.gguf");
        symlink(&real_file, &symlink_at_cache_path).unwrap();

        assert_eq!(
            CacheEntryHealth::new(symlink_at_cache_path.clone())
                .heal()
                .await
                .unwrap(),
            CacheEntryState::Vacant
        );
        assert!(
            tokio::fs::symlink_metadata(&symlink_at_cache_path)
                .await
                .is_err(),
            "the symlink itself must be removed, not followed"
        );
        assert_eq!(
            tokio::fs::read(&real_file).await.unwrap(),
            b"real model bytes",
            "healing must remove the symlink, never the file it points to"
        );
    }

    #[tokio::test]
    async fn heal_returns_vacant_when_absent() {
        let directory = TempDir::new().unwrap();

        assert_eq!(
            CacheEntryHealth::new(directory.path().join("missing.gguf"))
                .heal()
                .await
                .unwrap(),
            CacheEntryState::Vacant
        );
    }

    #[tokio::test]
    async fn heal_propagates_error_when_parent_is_a_file() {
        let directory = TempDir::new().unwrap();
        let file_in_place_of_parent = directory.path().join("not-a-directory");
        tokio::fs::write(&file_in_place_of_parent, b"blocker")
            .await
            .unwrap();

        assert!(
            CacheEntryHealth::new(file_in_place_of_parent.join("child.gguf"))
                .heal()
                .await
                .is_err(),
            "a non-NotFound stat failure must propagate instead of being treated as healed"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn heal_propagates_error_when_removal_fails() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TempDir::new().unwrap();
        let readonly_parent = directory.path().join("readonly");
        tokio::fs::create_dir(&readonly_parent).await.unwrap();
        let directory_at_cache_path = readonly_parent.join("model.gguf");
        tokio::fs::create_dir(&directory_at_cache_path)
            .await
            .unwrap();
        let mut permissions = tokio::fs::metadata(&readonly_parent)
            .await
            .unwrap()
            .permissions();
        permissions.set_mode(0o500);
        tokio::fs::set_permissions(&readonly_parent, permissions)
            .await
            .unwrap();

        let result = CacheEntryHealth::new(directory_at_cache_path).heal().await;

        let mut restored = tokio::fs::metadata(&readonly_parent)
            .await
            .unwrap()
            .permissions();
        restored.set_mode(0o700);
        tokio::fs::set_permissions(&readonly_parent, restored)
            .await
            .unwrap();

        assert!(
            result.is_err(),
            "a removal failure under a read-only parent must propagate"
        );
    }

    #[tokio::test]
    async fn is_cached_returns_true_for_regular_file() {
        let directory = TempDir::new().unwrap();
        let file_path = directory.path().join("model.gguf");
        tokio::fs::write(&file_path, b"model bytes").await.unwrap();

        assert!(CacheEntryHealth::new(file_path).is_cached().await.unwrap());
    }

    #[tokio::test]
    async fn is_cached_returns_false_when_absent() {
        let directory = TempDir::new().unwrap();

        assert!(
            !CacheEntryHealth::new(directory.path().join("missing.gguf"))
                .is_cached()
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn is_cached_returns_false_for_directory() {
        let directory = TempDir::new().unwrap();
        let directory_at_cache_path = directory.path().join("model.gguf");
        tokio::fs::create_dir(&directory_at_cache_path)
            .await
            .unwrap();

        assert!(
            !CacheEntryHealth::new(directory_at_cache_path)
                .is_cached()
                .await
                .unwrap(),
            "a directory occupying the cache path is not a valid cached model"
        );
    }

    #[tokio::test]
    async fn is_cached_propagates_error_when_parent_is_a_file() {
        let directory = TempDir::new().unwrap();
        let file_in_place_of_parent = directory.path().join("not-a-directory");
        tokio::fs::write(&file_in_place_of_parent, b"blocker")
            .await
            .unwrap();

        assert!(
            CacheEntryHealth::new(file_in_place_of_parent.join("child.gguf"))
                .is_cached()
                .await
                .is_err(),
            "a non-NotFound stat failure must propagate instead of reporting a cache miss"
        );
    }
}
