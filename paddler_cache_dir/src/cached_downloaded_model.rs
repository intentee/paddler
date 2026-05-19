use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use fslock::LockFile;
use sha2::Digest;
use sha2::Sha256;
use tokio::fs;
use url::Url;

use crate::cache_dir::CacheDir;
use crate::cached_downloaded_model_lock::CachedDownloadedModelLock;
use crate::download_lock_acquisition_error::DownloadLockAcquisitionError;

const DEFAULT_BASENAME: &str = "model.gguf";
const DOWNLOADED_MODELS_SUBDIR: &str = "downloaded-models";
const LOCK_FILE_NAME: &str = ".lock";

fn hex_lowercase(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        })
}

fn url_basename(parsed: &Url) -> String {
    parsed
        .path_segments()
        .and_then(|mut segments| {
            segments
                .rfind(|segment| !segment.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| DEFAULT_BASENAME.to_owned())
}

pub struct CachedDownloadedModel {
    pub cache_file_path: PathBuf,
    pub cache_subdir: PathBuf,
    pub lock_file_path: PathBuf,
}

impl CachedDownloadedModel {
    pub fn new(cache_dir: &CacheDir, url_string: &str) -> Result<Self> {
        let parsed =
            Url::parse(url_string).with_context(|| format!("Invalid URL '{url_string}'"))?;
        let cache_root = cache_dir.resolve()?;

        let digest = Sha256::digest(url_string.as_bytes());
        let hex_digest = hex_lowercase(&digest);
        let basename = url_basename(&parsed);

        let cache_subdir = cache_root.join(DOWNLOADED_MODELS_SUBDIR).join(hex_digest);
        let cache_file_path = cache_subdir.join(basename);
        let lock_file_path = cache_subdir.join(LOCK_FILE_NAME);

        Ok(Self {
            cache_file_path,
            cache_subdir,
            lock_file_path,
        })
    }

    pub async fn is_cached(&self) -> Result<bool, std::io::Error> {
        fs::try_exists(&self.cache_file_path).await
    }

    pub async fn ensure_cache_subdir_exists(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.cache_subdir).await
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
    use url::Url;

    use crate::cache_dir::CacheDir;
    use crate::cached_downloaded_model::CachedDownloadedModel;
    use crate::cached_downloaded_model::hex_lowercase;
    use crate::cached_downloaded_model::url_basename;

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
    fn basename_uses_last_path_segment() {
        let parsed = Url::parse("https://host.example/folder/model.gguf").unwrap();

        assert_eq!(url_basename(&parsed), "model.gguf");
    }

    #[test]
    fn basename_falls_back_to_model_gguf_when_path_empty() {
        let parsed = Url::parse("https://host.example/").unwrap();

        assert_eq!(url_basename(&parsed), "model.gguf");
    }

    #[test]
    fn basename_ignores_trailing_slash() {
        let parsed = Url::parse("https://host.example/folder/model.gguf/").unwrap();

        assert_eq!(url_basename(&parsed), "model.gguf");
    }

    #[test]
    fn cache_file_path_is_sha256_of_url_with_basename() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/folder/model.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();

        let cache_file_string = cached.cache_file_path.to_string_lossy().into_owned();
        let expected_hex = hex_lowercase(&Sha256::digest(url_string.as_bytes()));

        assert!(cache_file_string.contains("downloaded-models"));
        assert!(cache_file_string.ends_with(&format!("{}model.gguf", std::path::MAIN_SEPARATOR)));
        assert!(cache_file_string.contains(&expected_hex));
    }

    #[test]
    fn lock_file_path_is_dot_lock_in_same_dir_as_cache_file() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let cached =
            CachedDownloadedModel::new(&cache_dir, "https://host.example/model.gguf").unwrap();

        assert_eq!(
            cached.cache_file_path.parent(),
            Some(cached.cache_subdir.as_path())
        );
        assert_eq!(
            cached.lock_file_path.parent(),
            Some(cached.cache_subdir.as_path())
        );
        assert_eq!(
            cached
                .lock_file_path
                .file_name()
                .and_then(|name| name.to_str()),
            Some(".lock")
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

        assert!(
            result
                .unwrap_err()
                .is_another_process_downloading()
        );
    }

    #[test]
    fn new_returns_error_when_url_does_not_parse() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());

        let result = CachedDownloadedModel::new(&cache_dir, "not a url");

        assert!(result.is_err());
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
