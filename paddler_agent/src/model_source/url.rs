use std::io;
use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use url::Url;

use paddler_cache_dir::CacheDir;
use paddler_cache_dir::CachedDownloadedModel;
use paddler_cache_dir::DownloadLockAcquisitionError;
use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::progress_sink::ProgressSink;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::ModelPath;
use paddler_messaging::url_model_reference::UrlModelReference;

use crate::agent_issue_fix::AgentIssueFix;
use crate::desired_model_resolution::DesiredModelResolution;
use crate::resolves_model_source::ResolvesModelSource;
use crate::slot_aggregated_status::SlotAggregatedStatus;

#[cfg(unix)]
fn is_disk_full(error: &io::Error) -> bool {
    error.raw_os_error() == Some(28)
}

#[cfg(windows)]
fn is_disk_full(error: &io::Error) -> bool {
    error.raw_os_error() == Some(112)
}

fn classify_cache_io_error(url_string: &str, error: &io::Error) -> AgentIssue {
    let model_path = ModelPath {
        model_path: url_string.to_owned(),
    };

    if error.kind() == io::ErrorKind::PermissionDenied {
        AgentIssue::CacheDirectoryIsNotWritable(model_path)
    } else if is_disk_full(error) {
        AgentIssue::CacheStorageIsFull(model_path)
    } else {
        AgentIssue::ModelCacheIsCorrupted(model_path)
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

pub struct UrlModelSource(pub UrlModelReference);

#[async_trait]
impl ResolvesModelSource for UrlModelSource {
    async fn resolve(
        &self,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution> {
        let cache_dir = CacheDir::from_process_env();

        resolve_url_into_cache(&self.0.url, &cache_dir, slot_aggregated_status).await
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;
    use std::sync::Arc;

    use anyhow::anyhow;
    use paddler_cache_dir::CacheDir;
    use paddler_cache_dir::CachedDownloadedModel;
    use paddler_download_manager::download_error::DownloadError;
    use paddler_messaging::agent_issue::AgentIssue;
    use reqwest::StatusCode;
    use tempfile::TempDir;
    use tokio::io::AsyncBufReadExt as _;
    use tokio::io::AsyncWriteExt as _;
    use tokio::io::BufReader;
    use tokio::net::TcpListener;
    use url::Url;

    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::model_source::url::SlotAggregatedStatusSink;
    use crate::model_source::url::agent_issue_for;
    use crate::model_source::url::classify_cache_io_error;
    use crate::model_source::url::resolve_url_into_cache;
    use crate::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_download_manager::progress_sink::ProgressSink;
    use paddler_messaging::agent_issue_params::ModelPath;
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
        tokio::fs::write(&cached.cache_file_path, b"cached content")
            .await
            .unwrap();

        let resolution = resolve_url_into_cache(url_string, &cache_dir, fresh_status())
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
        let result = resolve_url_into_cache(url_string, &cache_dir, status.clone()).await;

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
        let result = resolve_url_into_cache(url_string, &cache_dir, status.clone()).await;

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
        let result = resolve_url_into_cache(url_string, &cache_dir, status.clone()).await;

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
        let result = resolve_url_into_cache(url_string, &cache_dir, status.clone()).await;

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
        tokio::fs::create_dir(&cached.lock_file_path).await.unwrap();

        let status = fresh_status();
        let result = resolve_url_into_cache(url_string, &cache_dir, status.clone()).await;

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
            source: io::Error::from_raw_os_error(28),
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
    fn classify_cache_io_error_maps_disk_full_errno_to_cache_storage_is_full() {
        #[cfg(unix)]
        const DISK_FULL_ERRNO: i32 = 28;
        #[cfg(windows)]
        const DISK_FULL_ERRNO: i32 = 112;

        let error = io::Error::from_raw_os_error(DISK_FULL_ERRNO);

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
        tokio::fs::write(directory.path().join("downloaded-models"), b"blocker")
            .await
            .unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/blocked.gguf";

        let status = fresh_status();
        let result = resolve_url_into_cache(url_string, &cache_dir, status.clone()).await;

        assert!(result.is_err(), "blocked cache subdir must produce an Err");
        assert!(
            status.has_issue_like(|issue| matches!(issue, AgentIssue::ModelCacheIsCorrupted(_)))
        );
    }

    #[test]
    fn sink_on_started_sets_download_status_and_clears_matching_download_issue() {
        let status = fresh_status();
        status.register_issue(AgentIssue::DownloadInterrupted(ModelPath {
            model_path: TEST_URL.to_owned(),
        }));

        let sink = SlotAggregatedStatusSink {
            basename: Some("m.gguf".to_owned()),
            slot_aggregated_status: status.clone(),
            url: TEST_URL.to_owned(),
        };

        sink.on_started(Some(500), 100);

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
    fn sink_on_chunk_increments_download_current() {
        let status = fresh_status();
        let sink = SlotAggregatedStatusSink {
            basename: None,
            slot_aggregated_status: status.clone(),
            url: TEST_URL.to_owned(),
        };

        sink.on_started(Some(1000), 0);
        sink.on_chunk(250);
        sink.on_chunk(125);

        let snapshot = status.make_snapshot().unwrap();
        assert_eq!(snapshot.download_current, 375);
    }

    #[test]
    fn sink_on_finished_resets_download_and_clears_matching_download_issue() {
        let status = fresh_status();
        status.register_issue(AgentIssue::DownloadInterrupted(ModelPath {
            model_path: TEST_URL.to_owned(),
        }));

        let sink = SlotAggregatedStatusSink {
            basename: Some("m.gguf".to_owned()),
            slot_aggregated_status: status.clone(),
            url: TEST_URL.to_owned(),
        };

        sink.on_started(Some(500), 200);
        sink.on_finished();

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

        let result =
            resolve_url_into_cache(url_string, &unresolvable_cache_dir(), fresh_status()).await;

        assert!(
            result.is_err(),
            "an unresolvable cache directory must produce an Err"
        );
    }

    async fn serve_single_ok_response(listener: TcpListener, body: Vec<u8>) {
        let (mut socket, _peer) = listener.accept().await.unwrap();
        let (reader_half, mut writer_half) = socket.split();
        let mut reader = BufReader::new(reader_half);

        loop {
            let mut header_line = String::new();
            let bytes_read = reader.read_line(&mut header_line).await.unwrap();
            if bytes_read == 0 || header_line == "\r\n" {
                break;
            }
        }

        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        writer_half.write_all(header.as_bytes()).await.unwrap();
        writer_half.write_all(&body).await.unwrap();
        writer_half.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn successful_download_resolves_to_cache_file_with_downloaded_contents() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url_string = format!("http://127.0.0.1:{port}/model.gguf");
        let body = b"downloaded model bytes".to_vec();
        let server = tokio::spawn(serve_single_ok_response(listener, body.clone()));

        let cached = CachedDownloadedModel::new(&cache_dir, &url_string).unwrap();
        let expected_path = cached.cache_file_path.clone();

        let resolution = resolve_url_into_cache(&url_string, &cache_dir, fresh_status())
            .await
            .unwrap();

        server.await.unwrap();

        assert!(matches!(
            resolution,
            DesiredModelResolution::Resolved(resolved_path) if resolved_path == expected_path
        ));
        assert_eq!(tokio::fs::read(&expected_path).await.unwrap(), body);
    }
}
