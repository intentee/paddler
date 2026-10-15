use std::io::Error;
use std::path::PathBuf;

use fslock::LockFile;
use sha2::Digest;
use sha2::Sha256;
use tokio::fs::create_dir_all;
use tokio::fs::try_exists;

use crate::cache_dir::CacheDir;
use crate::cache_dir_error::CacheDirError;
use crate::cached_downloaded_model_lock::CachedDownloadedModelLock;
use crate::download_lock_acquisition::DownloadLockAcquisition;

const DOWNLOADED_MODELS_SUBDIR: &str = "downloaded-models";

pub struct CachedDownloadedModel {
    pub cache_file_path: PathBuf,
    pub cache_subdir: PathBuf,
    pub lock_file_path: PathBuf,
}

impl CachedDownloadedModel {
    pub fn new(cache_dir: &CacheDir, url_string: &str) -> Result<Self, CacheDirError> {
        cache_dir.resolve().map(|cache_root| {
            let basename = format!("{:x}", Sha256::digest(url_string.as_bytes()));
            let cache_subdir = cache_root.join(DOWNLOADED_MODELS_SUBDIR);

            Self {
                cache_file_path: cache_subdir.join(&basename),
                lock_file_path: cache_subdir.join(format!("{basename}.lock")),
                cache_subdir,
            }
        })
    }

    pub async fn is_cached(&self) -> Result<bool, Error> {
        try_exists(&self.cache_file_path).await
    }

    pub async fn ensure_cache_subdir_exists(&self) -> Result<(), Error> {
        create_dir_all(&self.cache_subdir).await
    }

    pub fn try_acquire_download_lock(&self) -> Result<DownloadLockAcquisition, Error> {
        LockFile::open(&self.lock_file_path).and_then(|mut lock_file| {
            lock_file.try_lock().map(|acquired| {
                if acquired {
                    DownloadLockAcquisition::Acquired(CachedDownloadedModelLock::new(lock_file))
                } else {
                    DownloadLockAcquisition::HeldByAnotherProcess
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::io::ErrorKind;
    use std::mem::discriminant;
    use std::path::Path;

    use fslock::LockFile;
    use tempfile::TempDir;
    use tokio::fs::write;

    use crate::cache_dir::CacheDir;
    use crate::cache_dir_error::CacheDirError;
    use crate::cached_downloaded_model::CachedDownloadedModel;
    use crate::download_lock_acquisition::DownloadLockAcquisition;

    fn cache_dir_at(path: &Path) -> CacheDir {
        CacheDir {
            explicit: Some(path.to_path_buf()),
            home: None,
            xdg: None,
        }
    }

    #[test]
    fn cache_file_basename_is_the_sha256_hex_of_a_traversal_url() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://example.com/../../etc/passwd?token=secret";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();

        assert_eq!(
            cached.cache_file_path.file_name(),
            Some(OsStr::new(
                "2b8d7d17bb8ccf5e8250fc680e613b7c8e8179522fdb52f786dc6a1d593708f1"
            ))
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

        let expected_path = directory
            .path()
            .join("downloaded-models")
            .join("d211d40a16cf4462dec80daa95e529e1888e86811d721cafb2ff02549a85d57f");

        assert_eq!(cached.cache_file_path, expected_path);
    }

    #[test]
    fn lock_file_path_is_hex_dot_lock_next_to_cache_file() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/model.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();

        let expected_lock = directory
            .path()
            .join("downloaded-models")
            .join("5956957a5edf833e4bff4f1c67ac8bbe488602ecd5e659415a6cbbd14c4a91d2.lock");

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
        write(&cached.cache_file_path, b"cached").await.unwrap();

        assert!(cached.is_cached().await.unwrap());
    }

    #[tokio::test]
    async fn try_acquire_download_lock_succeeds_when_uncontested() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/model.gguf").unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();

        let _acquisition = cached.try_acquire_download_lock().unwrap();
        let mut competitor = LockFile::open(&cached.lock_file_path).unwrap();

        assert!(!competitor.try_lock().unwrap());
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

        let acquisition = cached.try_acquire_download_lock().unwrap();

        assert_eq!(
            discriminant(&acquisition),
            discriminant(&DownloadLockAcquisition::HeldByAnotherProcess)
        );
    }

    #[test]
    fn new_returns_error_when_cache_dir_cannot_resolve() {
        let unresolvable = CacheDir {
            explicit: None,
            home: None,
            xdg: None,
        };

        let result = CachedDownloadedModel::new(&unresolvable, "https://host.example/m.gguf");

        assert_eq!(
            result.err().as_ref().map(discriminant),
            Some(discriminant(&CacheDirError::HomeVariableUnset))
        );
    }

    #[tokio::test]
    async fn try_acquire_download_lock_returns_io_when_cache_subdir_missing() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/model.gguf").unwrap();

        let result = cached.try_acquire_download_lock();

        assert!(matches!(
            result,
            Err(lock_error) if lock_error.kind() == ErrorKind::NotFound
        ));
    }

    #[tokio::test]
    async fn lock_releases_on_drop_so_subsequent_acquire_succeeds() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/model.gguf").unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();

        let first_acquisition = cached.try_acquire_download_lock().unwrap();

        drop(first_acquisition);

        let _second_acquisition = cached.try_acquire_download_lock().unwrap();
        let mut competitor = LockFile::open(&cached.lock_file_path).unwrap();

        assert!(!competitor.try_lock().unwrap());
    }
}
