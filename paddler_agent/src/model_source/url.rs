use std::io;
use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use url::Url;

use paddler_cache_dir::cache_dir::CacheDir;
use paddler_cache_dir::cached_downloaded_model::CachedDownloadedModel;
use paddler_cache_dir::download_lock_acquisition_error::DownloadLockAcquisitionError;
use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::download_outcome::DownloadOutcome;
use paddler_download_manager::download_progress::DownloadProgress;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::url_model_reference::UrlModelReference;

use crate::agent_issue_fix::AgentIssueFix;
use crate::desired_model_resolution::DesiredModelResolution;
use crate::resolves_model_source::ResolvesModelSource;
use crate::slot_aggregated_status::SlotAggregatedStatus;

fn classify_cache_io_error(url_string: &str, error: &io::Error) -> AgentIssue {
    let model_path = ModelPath {
        model_path: url_string.to_owned(),
    };

    match error.kind() {
        io::ErrorKind::PermissionDenied => AgentIssue::CacheDirectoryIsNotWritable(model_path),
        io::ErrorKind::StorageFull => AgentIssue::CacheStorageIsFull(model_path),
        _ => AgentIssue::ModelCacheIsCorrupted(model_path),
    }
}

fn agent_issue_for(error: &DownloadError, url_string: &str) -> AgentIssue {
    let model_path = ModelPath {
        model_path: url_string.to_owned(),
    };

    match error {
        DownloadError::InvalidUrl { .. } | DownloadError::UnsupportedUrlScheme { .. } => {
            AgentIssue::DownloadUrlIsMalformed(model_path)
        }
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
        DownloadError::DownloadServerRejectedRequest { .. } => {
            AgentIssue::DownloadServerRejectedRequest(model_path)
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

struct SlotAggregatedStatusDownloadProgress {
    basename: Option<String>,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
    url: String,
}

impl SlotAggregatedStatusDownloadProgress {
    fn apply(&self, progress: DownloadProgress) {
        match progress {
            DownloadProgress::Started {
                already_downloaded_bytes,
                total_bytes,
            } => {
                self.slot_aggregated_status.set_download_status(
                    already_downloaded_bytes,
                    total_bytes,
                    self.basename.clone(),
                );
                self.slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelDownloadStarted(ModelPath {
                        model_path: self.url.clone(),
                    }));
            }
            DownloadProgress::ChunkWritten { byte_count } => {
                self.slot_aggregated_status
                    .increment_download_current(byte_count);
            }
            DownloadProgress::Finished => {
                self.slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelDownloadCompleted(ModelPath {
                        model_path: self.url.clone(),
                    }));
                self.slot_aggregated_status.reset_download();
            }
        }
    }
}

async fn resolve_url_into_cache(
    cancellation_token: &CancellationToken,
    url_string: &str,
    cache_dir: &CacheDir,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<DesiredModelResolution> {
    let parsed_url = match Url::parse(url_string) {
        Ok(url) => url,
        Err(parse_error) => {
            slot_aggregated_status.reset_download();
            slot_aggregated_status.register_issue(AgentIssue::DownloadUrlIsMalformed(ModelPath {
                model_path: url_string.to_owned(),
            }));

            return Err(
                anyhow::Error::new(parse_error).context(format!("Invalid URL '{url_string}'"))
            );
        }
    };

    if !matches!(parsed_url.scheme(), "http" | "https") {
        slot_aggregated_status.reset_download();
        slot_aggregated_status.register_issue(AgentIssue::DownloadUrlIsMalformed(ModelPath {
            model_path: url_string.to_owned(),
        }));

        return Err(anyhow!(
            "Unsupported URL scheme '{}' for '{url_string}'; only http and https are supported",
            parsed_url.scheme(),
        ));
    }

    let cached = CachedDownloadedModel::new(cache_dir, url_string)?;

    let is_cached = match cached.is_cached().await {
        Ok(value) => value,
        Err(io_error) => {
            slot_aggregated_status.reset_download();
            slot_aggregated_status.register_issue(classify_cache_io_error(url_string, &io_error));

            return Err(anyhow::Error::new(io_error));
        }
    };

    if is_cached {
        slot_aggregated_status.reset_download();
        slot_aggregated_status.register_fix(&AgentIssueFix::ModelDownloadCompleted(ModelPath {
            model_path: url_string.to_owned(),
        }));

        return Ok(DesiredModelResolution::Resolved(cached.cache_file_path));
    }

    if let Err(io_error) = cached.ensure_cache_subdir_exists().await {
        slot_aggregated_status.reset_download();
        slot_aggregated_status.register_issue(classify_cache_io_error(url_string, &io_error));

        return Err(anyhow::Error::new(io_error));
    }

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
            slot_aggregated_status.register_issue(classify_cache_io_error(url_string, &io_error));

            return Err(anyhow::Error::new(io_error));
        }
    };

    let basename = cached
        .cache_file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned);
    let download_progress = SlotAggregatedStatusDownloadProgress {
        basename,
        slot_aggregated_status: slot_aggregated_status.clone(),
        url: url_string.to_owned(),
    };
    let download_result = DownloadManager::new()?
        .download(
            cancellation_token,
            url_string,
            &cached.cache_file_path,
            &|progress| download_progress.apply(progress),
        )
        .await;

    match download_result {
        Ok(DownloadOutcome::Completed) => {
            Ok(DesiredModelResolution::Resolved(cached.cache_file_path))
        }
        Ok(DownloadOutcome::Cancelled) => Ok(DesiredModelResolution::Cancelled),
        Err(error) => {
            slot_aggregated_status.reset_download();
            slot_aggregated_status.register_issue(agent_issue_for(&error, url_string));

            Err(anyhow::Error::new(error))
        }
    }
}

pub struct UrlModelSource(pub UrlModelReference);

#[async_trait]
impl ResolvesModelSource for UrlModelSource {
    async fn resolve(
        &self,
        cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution> {
        let cache_dir = CacheDir::from_process_env();

        resolve_url_into_cache(
            cancellation_token,
            &self.0.url,
            &cache_dir,
            slot_aggregated_status,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;
    use std::sync::Arc;

    use anyhow::anyhow;
    use paddler_cache_dir::cache_dir::CacheDir;
    use paddler_cache_dir::cached_downloaded_model::CachedDownloadedModel;
    use paddler_download_manager::download_error::DownloadError;
    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::agent_issue::AgentIssue;
    use reqwest::StatusCode;
    use tempfile::TempDir;
    use tokio::fs::create_dir;
    use tokio::fs::read;
    use tokio::fs::write;
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::model_source::url::SlotAggregatedStatusDownloadProgress;
    use crate::model_source::url::agent_issue_for;
    use crate::model_source::url::classify_cache_io_error;
    use crate::model_source::url::resolve_url_into_cache;
    use crate::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_download_manager::download_progress::DownloadProgress;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;

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
    async fn cache_hit_returns_path_without_calling_download_manager() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/cached.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();
        write(&cached.cache_file_path, b"cached content")
            .await
            .unwrap();

        let resolution = resolve_url_into_cache(
            &CancellationToken::new(),
            url_string,
            &cache_dir,
            fresh_status(),
        )
        .await
        .unwrap();

        assert!(matches!(
            resolution,
            DesiredModelResolution::Resolved(resolved_path) if resolved_path == cached.cache_file_path
        ));
    }

    #[tokio::test]
    async fn malformed_url_registers_download_url_is_malformed() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "not a url";

        let status = fresh_status();
        let result = resolve_url_into_cache(
            &CancellationToken::new(),
            url_string,
            &cache_dir,
            status.clone(),
        )
        .await;

        assert!(result.is_err(), "malformed URL must produce an Err");
        assert!(
            status.has_issue(&AgentIssue::DownloadUrlIsMalformed(ModelPath {
                model_path: url_string.to_owned(),
            }))
        );
    }

    #[tokio::test]
    async fn unsupported_scheme_registers_download_url_is_malformed_without_creating_cache_state() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "ftp://example.invalid/m.gguf";

        let status = fresh_status();
        let result = resolve_url_into_cache(
            &CancellationToken::new(),
            url_string,
            &cache_dir,
            status.clone(),
        )
        .await;

        assert!(result.is_err(), "unsupported scheme must produce an Err");
        assert!(
            status.has_issue(&AgentIssue::DownloadUrlIsMalformed(ModelPath {
                model_path: url_string.to_owned(),
            }))
        );
        assert!(
            !directory.path().join("downloaded-models").exists(),
            "no cache subdirectory must be created for an unsupported scheme"
        );
    }

    #[tokio::test]
    async fn lock_contention_registers_cache_cannot_acquire_lock() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/contended.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();

        let _blocker = cached.try_acquire_download_lock().unwrap();

        let status = fresh_status();
        let result = resolve_url_into_cache(
            &CancellationToken::new(),
            url_string,
            &cache_dir,
            status.clone(),
        )
        .await;

        assert!(result.is_err(), "lock contention must produce an Err");
        assert!(
            status.has_issue(&AgentIssue::CacheCannotAcquireLock(ModelPath {
                model_path: url_string.to_owned(),
            }))
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cache_subdir_creation_failure_registers_model_cache_is_corrupted() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/subdir-blocked.gguf";
        let subdir_path = directory.path().join("downloaded-models");
        symlink(directory.path().join("missing-target"), &subdir_path).unwrap();

        let status = fresh_status();
        let result = resolve_url_into_cache(
            &CancellationToken::new(),
            url_string,
            &cache_dir,
            status.clone(),
        )
        .await;

        assert!(
            result.is_err(),
            "a non-directory at the cache subdir path must produce an Err"
        );
        assert!(
            status.has_issue_like(|issue| matches!(issue, AgentIssue::ModelCacheIsCorrupted(_)))
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn lock_open_io_error_registers_model_cache_is_corrupted() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/lock-as-directory.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();
        create_dir(&cached.lock_file_path).await.unwrap();

        let status = fresh_status();
        let result = resolve_url_into_cache(
            &CancellationToken::new(),
            url_string,
            &cache_dir,
            status.clone(),
        )
        .await;

        assert!(
            result.is_err(),
            "an unopenable lock path must produce an Err"
        );
        assert!(
            status.has_issue_like(|issue| matches!(issue, AgentIssue::ModelCacheIsCorrupted(_)))
        );
    }

    fn test_model_path() -> ModelPath {
        ModelPath {
            model_path: TEST_URL.to_owned(),
        }
    }

    #[test]
    fn invalid_url_maps_to_download_url_is_malformed() {
        let parse_error = Url::parse("not a url").err().unwrap();
        let error = DownloadError::InvalidUrl {
            url: "not a url".to_owned(),
            source: parse_error,
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadUrlIsMalformed(test_model_path())
        );
    }

    #[test]
    fn unsupported_url_scheme_maps_to_download_url_is_malformed() {
        let error = DownloadError::UnsupportedUrlScheme {
            url: TEST_URL.to_owned(),
            scheme: "ftp".to_owned(),
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadUrlIsMalformed(test_model_path())
        );
    }

    #[test]
    fn not_found_maps_to_model_does_not_exist_at_url() {
        let error = DownloadError::NotFound {
            url: TEST_URL.to_owned(),
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::ModelDoesNotExistAtUrl(test_model_path())
        );
    }

    #[test]
    fn permission_denied_maps_to_download_server_denied_access() {
        let error = DownloadError::PermissionDenied {
            url: TEST_URL.to_owned(),
            status: StatusCode::FORBIDDEN,
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadServerDeniedAccess(test_model_path())
        );
    }

    #[test]
    fn partial_file_stale_maps_to_model_cache_is_corrupted() {
        let error = DownloadError::PartialFileStale {
            url: TEST_URL.to_owned(),
            partial_path: PathBuf::from("/tmp/stale.partial"),
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::ModelCacheIsCorrupted(test_model_path())
        );
    }

    #[test]
    fn download_server_is_unreachable_maps_to_agent_issue() {
        let error = DownloadError::DownloadServerIsUnreachable {
            url: TEST_URL.to_owned(),
            source: anyhow!("connection refused"),
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadServerIsUnreachable(test_model_path())
        );
    }

    #[test]
    fn download_server_errored_maps_to_agent_issue() {
        let error = DownloadError::DownloadServerErrored {
            url: TEST_URL.to_owned(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadServerErrored(test_model_path())
        );
    }

    #[test]
    fn download_server_rejected_request_maps_to_agent_issue() {
        let error = DownloadError::DownloadServerRejectedRequest {
            url: TEST_URL.to_owned(),
            status: StatusCode::BAD_REQUEST,
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadServerRejectedRequest(test_model_path())
        );
    }

    #[test]
    fn download_interrupted_maps_to_agent_issue() {
        let error = DownloadError::DownloadInterrupted {
            url: TEST_URL.to_owned(),
            source: anyhow!("stream dropped"),
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::DownloadInterrupted(test_model_path())
        );
    }

    #[test]
    fn cache_permission_denied_maps_to_cache_directory_is_not_writable() {
        let error = DownloadError::CachePermissionDenied {
            path: PathBuf::from("/tmp/locked/model.partial"),
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::CacheDirectoryIsNotWritable(test_model_path())
        );
    }

    #[test]
    fn cache_disk_full_maps_to_cache_storage_is_full() {
        let error = DownloadError::CacheDiskFull {
            path: PathBuf::from("/tmp/full/model.partial"),
            source: io::Error::from(io::ErrorKind::StorageFull),
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::CacheStorageIsFull(test_model_path())
        );
    }

    #[test]
    fn io_maps_to_model_cache_is_corrupted() {
        let error = DownloadError::Io {
            path: PathBuf::from("/tmp/anywhere/model.partial"),
            source: io::Error::from(io::ErrorKind::NotFound),
        };

        assert_eq!(
            agent_issue_for(&error, TEST_URL),
            AgentIssue::ModelCacheIsCorrupted(test_model_path())
        );
    }

    #[test]
    fn classify_cache_io_error_maps_permission_denied_to_cache_directory_is_not_writable() {
        let error = io::Error::from(io::ErrorKind::PermissionDenied);

        assert_eq!(
            classify_cache_io_error(TEST_URL, &error),
            AgentIssue::CacheDirectoryIsNotWritable(test_model_path())
        );
    }

    #[test]
    fn classify_cache_io_error_maps_storage_full_to_cache_storage_is_full() {
        let error = io::Error::from(io::ErrorKind::StorageFull);

        assert_eq!(
            classify_cache_io_error(TEST_URL, &error),
            AgentIssue::CacheStorageIsFull(test_model_path())
        );
    }

    #[test]
    fn classify_cache_io_error_falls_back_to_model_cache_is_corrupted() {
        let error = io::Error::from(io::ErrorKind::NotFound);

        assert_eq!(
            classify_cache_io_error(TEST_URL, &error),
            AgentIssue::ModelCacheIsCorrupted(test_model_path())
        );
    }

    #[tokio::test]
    async fn ensure_cache_subdir_failure_registers_model_cache_is_corrupted() {
        let directory = TempDir::new().unwrap();
        write(directory.path().join("downloaded-models"), b"blocker")
            .await
            .unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/blocked.gguf";

        let status = fresh_status();
        let result = resolve_url_into_cache(
            &CancellationToken::new(),
            url_string,
            &cache_dir,
            status.clone(),
        )
        .await;

        assert!(result.is_err(), "blocked cache subdir must produce an Err");
        assert!(
            status.has_issue_like(|issue| matches!(issue, AgentIssue::ModelCacheIsCorrupted(_)))
        );
    }

    #[test]
    fn applying_started_sets_download_status_and_clears_matching_download_issue() {
        let status = fresh_status();
        status.register_issue(AgentIssue::DownloadInterrupted(ModelPath {
            model_path: TEST_URL.to_owned(),
        }));

        let download_progress = SlotAggregatedStatusDownloadProgress {
            basename: Some("m.gguf".to_owned()),
            slot_aggregated_status: status.clone(),
            url: TEST_URL.to_owned(),
        };

        download_progress.apply(DownloadProgress::Started {
            already_downloaded_bytes: 100,
            total_bytes: Some(500),
        });

        let snapshot = status.make_snapshot().unwrap();
        assert_eq!(snapshot.download_current, 100);
        assert_eq!(snapshot.download_total, 500);
        assert!(!snapshot.download_indeterminate);
        assert_eq!(snapshot.download_filename, Some("m.gguf".to_owned()));
        assert!(
            !status.has_issue(&AgentIssue::DownloadInterrupted(ModelPath {
                model_path: TEST_URL.to_owned(),
            }))
        );
    }

    #[test]
    fn applying_chunk_written_increments_download_current() {
        let status = fresh_status();
        let download_progress = SlotAggregatedStatusDownloadProgress {
            basename: None,
            slot_aggregated_status: status.clone(),
            url: TEST_URL.to_owned(),
        };

        download_progress.apply(DownloadProgress::Started {
            already_downloaded_bytes: 0,
            total_bytes: Some(1000),
        });
        download_progress.apply(DownloadProgress::ChunkWritten { byte_count: 250 });
        download_progress.apply(DownloadProgress::ChunkWritten { byte_count: 125 });

        let snapshot = status.make_snapshot().unwrap();
        assert_eq!(snapshot.download_current, 375);
    }

    #[test]
    fn applying_finished_resets_download_and_clears_matching_download_issue() {
        let status = fresh_status();
        status.register_issue(AgentIssue::DownloadInterrupted(ModelPath {
            model_path: TEST_URL.to_owned(),
        }));

        let download_progress = SlotAggregatedStatusDownloadProgress {
            basename: Some("m.gguf".to_owned()),
            slot_aggregated_status: status.clone(),
            url: TEST_URL.to_owned(),
        };

        download_progress.apply(DownloadProgress::Started {
            already_downloaded_bytes: 200,
            total_bytes: Some(500),
        });
        download_progress.apply(DownloadProgress::Finished);

        let snapshot = status.make_snapshot().unwrap();
        assert_eq!(snapshot.download_current, 0);
        assert_eq!(snapshot.download_total, 0);
        assert!(snapshot.download_indeterminate);
        assert_eq!(snapshot.download_filename, None);
        assert!(
            !status.has_issue(&AgentIssue::DownloadInterrupted(ModelPath {
                model_path: TEST_URL.to_owned(),
            }))
        );
    }

    fn unresolvable_cache_dir() -> CacheDir {
        #[cfg(unix)]
        {
            CacheDir {
                explicit: None,
                home: None,
                xdg: None,
            }
        }
        #[cfg(windows)]
        {
            CacheDir {
                explicit: None,
                localappdata: None,
                userprofile: None,
            }
        }
    }

    #[tokio::test]
    async fn cache_path_resolution_failure_propagates_error() {
        let url_string = "https://host.example/unresolvable.gguf";

        let result = resolve_url_into_cache(
            &CancellationToken::new(),
            url_string,
            &unresolvable_cache_dir(),
            fresh_status(),
        )
        .await;

        assert!(
            result.is_err(),
            "an unresolvable cache directory must produce an Err"
        );
    }

    #[tokio::test]
    async fn successful_download_resolves_to_cache_file_with_downloaded_contents() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());

        let body = b"downloaded model bytes".to_vec();
        let fixture = LocalHttpFixture::start(FixtureResponse::Ok(body.clone()))
            .await
            .unwrap();
        let url_string = fixture.url("/model.gguf");

        let cached = CachedDownloadedModel::new(&cache_dir, &url_string).unwrap();
        let expected_path = cached.cache_file_path.clone();

        let resolution = resolve_url_into_cache(
            &CancellationToken::new(),
            &url_string,
            &cache_dir,
            fresh_status(),
        )
        .await
        .unwrap();

        assert_eq!(fixture.request_count(), 1);
        assert!(matches!(
            resolution,
            DesiredModelResolution::Resolved(resolved_path) if resolved_path == expected_path
        ));
        assert_eq!(read(&expected_path).await.unwrap(), body);
    }

    #[tokio::test]
    async fn a_cancelled_token_makes_the_url_download_error_without_registering_an_issue() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "http://127.0.0.1:1/model.gguf".to_owned();
        let status = fresh_status();
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        let result =
            resolve_url_into_cache(&cancellation_token, &url_string, &cache_dir, status.clone())
                .await;

        assert!(
            matches!(result, Ok(DesiredModelResolution::Cancelled)),
            "a cancelled download must report cancellation as an outcome, not an error"
        );
        assert!(
            status.make_snapshot().unwrap().issues.is_empty(),
            "a cancelled download must not register a slot issue"
        );
    }
}
