use std::io;
use std::path::PathBuf;

use tokio::fs;

pub struct CacheEntryValidator {
    path: PathBuf,
}

impl CacheEntryValidator {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn is_valid(&self) -> Result<bool, io::Error> {
        match fs::metadata(&self.path).await {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::cache_entry_validator::CacheEntryValidator;

    #[tokio::test]
    async fn is_valid_returns_true_for_regular_file() {
        let directory = TempDir::new().unwrap();
        let file_path = directory.path().join("model.gguf");
        tokio::fs::write(&file_path, b"model bytes").await.unwrap();

        let validator = CacheEntryValidator::new(file_path);

        assert!(validator.is_valid().await.unwrap());
    }

    #[tokio::test]
    async fn is_valid_returns_false_for_directory() {
        let directory = TempDir::new().unwrap();
        let directory_at_cache_path = directory.path().join("model.gguf");
        tokio::fs::create_dir(&directory_at_cache_path)
            .await
            .unwrap();

        let validator = CacheEntryValidator::new(directory_at_cache_path);

        assert!(
            !validator.is_valid().await.unwrap(),
            "a directory occupying the cache file path is not a valid cached model"
        );
    }

    #[tokio::test]
    async fn is_valid_returns_false_when_absent() {
        let directory = TempDir::new().unwrap();
        let validator = CacheEntryValidator::new(directory.path().join("missing.gguf"));

        assert!(!validator.is_valid().await.unwrap());
    }

    #[tokio::test]
    async fn is_valid_propagates_error_when_parent_is_a_file() {
        let directory = TempDir::new().unwrap();
        let file_in_place_of_parent = directory.path().join("not-a-directory");
        tokio::fs::write(&file_in_place_of_parent, b"blocker")
            .await
            .unwrap();
        let validator = CacheEntryValidator::new(file_in_place_of_parent.join("child.gguf"));

        assert!(
            validator.is_valid().await.is_err(),
            "a non-NotFound stat failure must propagate instead of reporting a cache miss"
        );
    }
}
