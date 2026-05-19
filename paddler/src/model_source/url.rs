use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;

use paddler_cache_dir::CacheDir;
use paddler_cache_dir::CachedDownloadedModel;
use paddler_cache_dir::DownloadLockAcquisitionError;
use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::progress_sink::ProgressSink;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_issue_params::ModelPath;
use paddler_types::url_model_reference::UrlModelReference;

use crate::agent_issue_fix::AgentIssueFix;
use crate::desired_model_resolution::DesiredModelResolution;
use crate::resolves_model_source::ResolvesModelSource;
use crate::slot_aggregated_status::SlotAggregatedStatus;

fn agent_issue_for(error: &DownloadError, url_string: &str) -> AgentIssue {
    let model_path = ModelPath {
        model_path: url_string.to_owned(),
    };

    match error {
        DownloadError::InvalidUrl { .. } => AgentIssue::DownloadUrlIsMalformed(model_path),
        DownloadError::NotFound { .. } => AgentIssue::ModelDoesNotExistAtUrl(model_path),
        DownloadError::PermissionDenied { .. } => {
            AgentIssue::DownloadServerDeniedAccess(model_path)
        }
        DownloadError::DownloadServerIsUnreachable { .. } => {
            AgentIssue::DownloadServerIsUnreachable(model_path)
        }
        DownloadError::DownloadServerErrored { .. } => {
            AgentIssue::DownloadServerErrored(model_path)
        }
        DownloadError::DownloadInterrupted { .. } => AgentIssue::DownloadInterrupted(model_path),
        DownloadError::CachePermissionDenied { .. } => {
            AgentIssue::CacheDirectoryIsNotWritable(model_path)
        }
        DownloadError::CacheDiskFull { .. } => AgentIssue::CacheStorageIsFull(model_path),
        DownloadError::PartialFileStale { .. } | DownloadError::Io { .. } => {
            AgentIssue::ModelCacheIsCorrupted(model_path)
        }
    }
}

struct SlotAggregatedStatusSink {
    basename: Option<String>,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
    url: String,
}

impl ProgressSink for SlotAggregatedStatusSink {
    fn on_started(&self, total_bytes: Option<u64>, already_downloaded: u64) {
        self.slot_aggregated_status.set_download_status(
            already_downloaded,
            total_bytes,
            self.basename.clone(),
        );
        self.slot_aggregated_status
            .register_fix(&AgentIssueFix::ModelDownloadStarted(ModelPath {
                model_path: self.url.clone(),
            }));
    }

    fn on_chunk(&self, additional_bytes: u64) {
        self.slot_aggregated_status
            .increment_download_current(additional_bytes);
    }

    fn on_finished(&self) {
        self.slot_aggregated_status
            .register_fix(&AgentIssueFix::ModelDownloadCompleted(ModelPath {
                model_path: self.url.clone(),
            }));
        self.slot_aggregated_status.reset_download();
    }
}

async fn resolve_url_into_cache(
    url_string: &str,
    cache_dir: &CacheDir,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<DesiredModelResolution> {
    let cached = CachedDownloadedModel::new(cache_dir, url_string)?;

    if cached.is_cached().await? {
        slot_aggregated_status.reset_download();
        slot_aggregated_status.register_fix(&AgentIssueFix::ModelDownloadCompleted(ModelPath {
            model_path: url_string.to_owned(),
        }));

        return Ok(DesiredModelResolution::Resolved(cached.cache_file_path));
    }

    cached.ensure_cache_subdir_exists().await?;

    let _lock_guard = match cached.try_acquire_download_lock() {
        Ok(guard) => guard,
        Err(DownloadLockAcquisitionError::AnotherProcessIsDownloading) => {
            slot_aggregated_status.reset_download();
            slot_aggregated_status.register_issue(AgentIssue::CacheCannotAcquireLock(ModelPath {
                model_path: url_string.to_owned(),
            }));

            return Err(anyhow!(
                "Another agent on this host is currently downloading '{url_string}'"
            ));
        }
        Err(DownloadLockAcquisitionError::Io(io_error)) => {
            slot_aggregated_status.reset_download();

            return Err(anyhow::Error::new(io_error));
        }
    };

    let basename = cached
        .cache_file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned);
    let sink: Arc<dyn ProgressSink> = Arc::new(SlotAggregatedStatusSink {
        basename,
        slot_aggregated_status: slot_aggregated_status.clone(),
        url: url_string.to_owned(),
    });

    match DownloadManager::new()?
        .download(url_string, &cached.cache_file_path, sink)
        .await
    {
        Ok(()) => Ok(DesiredModelResolution::Resolved(cached.cache_file_path)),
        Err(error) => {
            slot_aggregated_status.reset_download();
            slot_aggregated_status.register_issue(agent_issue_for(&error, url_string));

            Err(anyhow::Error::new(error))
        }
    }
}

#[async_trait]
impl ResolvesModelSource for UrlModelReference {
    async fn resolve(
        &self,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution> {
        let cache_dir = CacheDir::from_process_env();

        resolve_url_into_cache(&self.url, &cache_dir, slot_aggregated_status).await
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;
    use std::sync::Arc;

    use anyhow::Context as _;
    use anyhow::Result;
    use anyhow::anyhow;
    use paddler_cache_dir::CacheDir;
    use paddler_cache_dir::CachedDownloadedModel;
    use paddler_download_manager::download_error::DownloadError;
    use paddler_types::agent_issue::AgentIssue;
    use reqwest::StatusCode;
    use tempfile::TempDir;
    use url::Url;

    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::model_source::url::agent_issue_for;
    use crate::model_source::url::resolve_url_into_cache;
    use crate::slot_aggregated_status::SlotAggregatedStatus;

    const TEST_URL: &str = "https://example.com/m.gguf";

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

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

    #[tokio::test]
    async fn cache_hit_returns_path_without_calling_download_manager() -> Result<()> {
        let directory = TempDir::new()?;
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/cached.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string)?;
        cached.ensure_cache_subdir_exists().await?;
        tokio::fs::write(&cached.cache_file_path, b"cached content").await?;

        let resolution =
            resolve_url_into_cache(url_string, &cache_dir, fresh_status()).await?;

        match resolution {
            DesiredModelResolution::Resolved(path) => {
                assert_eq!(path, cached.cache_file_path);
            }
            other => return Err(anyhow!("expected Resolved, got {other:?}")),
        }

        Ok(())
    }

    #[tokio::test]
    async fn lock_contention_registers_cache_cannot_acquire_lock() -> Result<()> {
        let directory = TempDir::new()?;
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/contended.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string)?;
        cached.ensure_cache_subdir_exists().await?;

        let _blocker = cached.try_acquire_download_lock()?;

        let status = fresh_status();
        let result = resolve_url_into_cache(url_string, &cache_dir, status.clone()).await;

        assert!(result.is_err(), "lock contention must produce an Err");
        assert!(status.has_issue(&AgentIssue::CacheCannotAcquireLock(
            paddler_types::agent_issue_params::ModelPath {
                model_path: url_string.to_owned(),
            },
        )));

        Ok(())
    }

    #[test]
    fn invalid_url_maps_to_download_url_is_malformed() -> Result<()> {
        let parse_error = Url::parse("not a url")
            .err()
            .context("'not a url' should not parse")?;
        let error = DownloadError::InvalidUrl {
            url: "not a url".to_owned(),
            source: parse_error,
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadUrlIsMalformed(_)
        ));

        Ok(())
    }

    #[test]
    fn not_found_maps_to_model_does_not_exist_at_url() {
        let error = DownloadError::NotFound {
            url: TEST_URL.to_owned(),
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::ModelDoesNotExistAtUrl(_)
        ));
    }

    #[test]
    fn permission_denied_maps_to_download_server_denied_access() {
        let error = DownloadError::PermissionDenied {
            url: TEST_URL.to_owned(),
            status: StatusCode::FORBIDDEN,
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadServerDeniedAccess(_)
        ));
    }

    #[test]
    fn partial_file_stale_maps_to_model_cache_is_corrupted() {
        let error = DownloadError::PartialFileStale {
            url: TEST_URL.to_owned(),
            partial_path: PathBuf::from("/tmp/stale.partial"),
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::ModelCacheIsCorrupted(_)
        ));
    }

    #[test]
    fn download_server_is_unreachable_maps_to_agent_issue() {
        let error = DownloadError::DownloadServerIsUnreachable {
            url: TEST_URL.to_owned(),
            source: anyhow!("connection refused"),
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadServerIsUnreachable(_)
        ));
    }

    #[test]
    fn download_server_errored_maps_to_agent_issue() {
        let error = DownloadError::DownloadServerErrored {
            url: TEST_URL.to_owned(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadServerErrored(_)
        ));
    }

    #[test]
    fn download_interrupted_maps_to_agent_issue() {
        let error = DownloadError::DownloadInterrupted {
            url: TEST_URL.to_owned(),
            source: anyhow!("stream dropped"),
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadInterrupted(_)
        ));
    }

    #[test]
    fn cache_permission_denied_maps_to_cache_directory_is_not_writable() {
        let error = DownloadError::CachePermissionDenied {
            path: PathBuf::from("/tmp/locked/model.partial"),
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::CacheDirectoryIsNotWritable(_)
        ));
    }

    #[test]
    fn cache_disk_full_maps_to_cache_storage_is_full() {
        let error = DownloadError::CacheDiskFull {
            path: PathBuf::from("/tmp/full/model.partial"),
            source: io::Error::from_raw_os_error(28),
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::CacheStorageIsFull(_)
        ));
    }

    #[test]
    fn io_maps_to_model_cache_is_corrupted() {
        let error = DownloadError::Io {
            path: PathBuf::from("/tmp/anywhere/model.partial"),
            source: io::Error::from(io::ErrorKind::NotFound),
        };

        assert!(matches!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::ModelCacheIsCorrupted(_)
        ));
    }
}
