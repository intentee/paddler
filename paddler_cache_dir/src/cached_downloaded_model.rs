use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::Result;
use fslock::LockFile;
use sha2::Digest;
use sha2::Sha256;
use tokio::fs::create_dir_all;
use tokio::fs::try_exists;

use crate::cache_dir::CacheDir;
use crate::cached_downloaded_model_lock::CachedDownloadedModelLock;
use crate::download_lock_acquisition_error::DownloadLockAcquisitionError;

const DOWNLOADED_MODELS_SUBDIR: &str = "downloaded-models";

fn hex_lowercase(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        })
}

pub struct CachedDownloadedModel {
    pub cache_file_path: PathBuf,
    pub cache_subdir: PathBuf,
    pub lock_file_path: PathBuf,
}

impl CachedDownloadedModel {
    pub fn new(cache_dir: &CacheDir, url_string: &str) -> Result<Self> {
        let cache_root = cache_dir.resolve()?;
        let basename = hex_lowercase(&Sha256::digest(url_string.as_bytes()));

        let cache_subdir = cache_root.join(DOWNLOADED_MODELS_SUBDIR);
        let cache_file_path = cache_subdir.join(&basename);
        let lock_file_path = cache_subdir.join(format!("{basename}.lock"));

        Ok(Self {
            cache_file_path,
            cache_subdir,
            lock_file_path,
        })
    }

    pub async fn is_cached(&self) -> Result<bool, std::io::Error> {
        try_exists(&self.cache_file_path).await
    }

    pub async fn ensure_cache_subdir_exists(&self) -> Result<(), std::io::Error> {
        create_dir_all(&self.cache_subdir).await
    }

    pub fn try_acquire_download_lock(
        &self,
    ) -> Result<CachedDownloadedModelLock, DownloadLockAcquisitionError> {
        let (acquired, lock_file) = LockFile::open(&self.lock_file_path)
            .and_then(|mut file| file.try_lock().map(|acquired| (acquired, file)))?;
        if acquired {
            Ok(CachedDownloadedModelLock::new(lock_file))
        } else {
            Err(DownloadLockAcquisitionError::AnotherProcessIsDownloading)
        }
    }
}

#[cfg(test)]
mod tests {
    use fslock::LockFile;
    use sha2::Digest;
    use sha2::Sha256;
    use tempfile::TempDir;

    use crate::cache_dir::CacheDir;
    use crate::cached_downloaded_model::CachedDownloadedModel;
    use crate::cached_downloaded_model::hex_lowercase;

    fn cache_dir_at(path: &std::path::Path) -> CacheDir {
        #[cfg(unix)]
        {
            CacheDir {
                explicit: Some(path.to_string_lossy().into_owned()),
                home: None,
                xdg: None,
            }
        }
        #[cfg(windows)]
        {
            CacheDir {
                explicit: Some(path.to_string_lossy().into_owned()),
                localappdata: None,
                userprofile: None,
            }
        }
    }

    #[test]
    fn cache_file_basename_is_only_lowercase_hex_for_traversal_url() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://example.com/../../etc/passwd?token=secret";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();

        let file_name = cached
            .cache_file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap();

        assert_eq!(file_name.len(), 64, "SHA-256 hex is 64 chars");
        assert!(
            file_name
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "basename {file_name:?} must be lowercase hex only"
        );
    }

    #[test]
    fn cache_file_path_for_traversal_url_stays_directly_under_downloaded_models() {
        let traversal_urls = [
            "https://example.com/..",
            "https://example.com/../../etc/passwd",
            "https://example.com//etc//passwd",
            "https://example.com/foo%2Fbar",
            "https://example.com/",
        ];

        for url_string in traversal_urls {
            let directory = TempDir::new().unwrap();
            let cache_dir = cache_dir_at(directory.path());
            let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();
            let expected_parent = directory.path().join("downloaded-models");

            assert_eq!(
                cached.cache_file_path.parent(),
                Some(expected_parent.as_path()),
                "URL {url_string:?} produced cache file outside downloaded-models"
            );
        }
    }

    #[test]
    fn cache_file_path_is_sha256_hex_under_downloaded_models() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/folder/model.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();

        let expected_hex = hex_lowercase(&Sha256::digest(url_string.as_bytes()));
        let expected_path = directory
            .path()
            .join("downloaded-models")
            .join(&expected_hex);

        assert_eq!(cached.cache_file_path, expected_path);
    }

    #[test]
    fn lock_file_path_is_hex_dot_lock_next_to_cache_file() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/model.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();

        let expected_hex = hex_lowercase(&Sha256::digest(url_string.as_bytes()));
        let expected_lock = directory
            .path()
            .join("downloaded-models")
            .join(format!("{expected_hex}.lock"));

        assert_eq!(cached.lock_file_path, expected_lock);
        assert_eq!(
            cached.cache_file_path.parent(),
            cached.lock_file_path.parent()
        );
    }

    #[tokio::test]
    async fn is_cached_returns_false_when_cache_file_absent() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/missing.gguf").unwrap();

        assert!(!cached.is_cached().await.unwrap());
    }

    #[tokio::test]
    async fn is_cached_returns_true_when_cache_file_present() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/present.gguf").unwrap();

        cached.ensure_cache_subdir_exists().await.unwrap();
        tokio::fs::write(&cached.cache_file_path, b"cached")
            .await
            .unwrap();

        assert!(cached.is_cached().await.unwrap());
    }

    #[tokio::test]
    async fn try_acquire_download_lock_succeeds_when_uncontested() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/model.gguf").unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();

        let _guard = cached.try_acquire_download_lock().unwrap();
    }

    #[tokio::test]
    async fn try_acquire_download_lock_returns_another_process_when_locked() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/model.gguf").unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();

        let mut blocker = LockFile::open(&cached.lock_file_path).unwrap();
        let blocker_acquired = blocker.try_lock().unwrap();
        assert!(blocker_acquired, "blocker must acquire the lock first");

        let result = cached.try_acquire_download_lock();

        assert!(result.unwrap_err().is_another_process_downloading());
    }

    #[test]
    fn new_returns_error_when_cache_dir_cannot_resolve() {
        let unresolvable;
        #[cfg(unix)]
        {
            unresolvable = CacheDir {
                explicit: None,
                home: None,
                xdg: None,
            };
        }
        #[cfg(windows)]
        {
            unresolvable = CacheDir {
                explicit: None,
                localappdata: None,
                userprofile: None,
            };
        }

        let result = CachedDownloadedModel::new(&unresolvable, "https://host.example/m.gguf");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn try_acquire_download_lock_returns_io_when_cache_subdir_missing() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/model.gguf").unwrap();

        let result = cached.try_acquire_download_lock();

        assert!(result.unwrap_err().is_io());
    }

    #[tokio::test]
    async fn lock_releases_on_drop_so_subsequent_acquire_succeeds() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/model.gguf").unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();

        {
            let _guard = cached.try_acquire_download_lock().unwrap();
        }

        let _second_guard = cached.try_acquire_download_lock().unwrap();
    }
}
