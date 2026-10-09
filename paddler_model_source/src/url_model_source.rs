use std::io;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_cache_dir::cached_downloaded_model::CachedDownloadedModel;
use paddler_cache_dir::download_lock_acquisition::DownloadLockAcquisition;
use paddler_download_manager::download_error::DownloadError;
use paddler_download_manager::download_manager::DownloadManager;
use paddler_download_manager::download_outcome::DownloadOutcome;
use paddler_download_manager::download_url::DownloadUrl;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::model_download_status::ModelDownloadStatus;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::download_error_agent_issue::download_error_agent_issue;
use crate::model_source_error::ModelSourceError;
use crate::resolves_model_source::ResolvesModelSource;
use crate::url_download_progress::UrlDownloadProgress;

pub struct UrlModelSource(pub DownloadUrl);

impl UrlModelSource {
    fn model_path(&self) -> ModelPath {
        ModelPath {
            model_path: self.0.as_str().to_owned(),
        }
    }

    fn fail_download(
        &self,
        slot_aggregated_status: &SlotAggregatedStatus,
        download_error: DownloadError,
    ) -> ModelSourceError {
        slot_aggregated_status.fail_download(download_error_agent_issue(
            &download_error,
            self.model_path(),
        ));

        ModelSourceError::DownloadFailed {
            url: self.0.as_str().to_owned(),
            source: download_error,
        }
    }

    fn fail_cache_access(
        &self,
        slot_aggregated_status: &SlotAggregatedStatus,
        path: &Path,
        io_error: io::Error,
    ) -> ModelSourceError {
        self.fail_download(
            slot_aggregated_status,
            DownloadError::cache_failure(path.to_path_buf(), io_error),
        )
    }

    async fn resolve_into_cache(
        &self,
        cancellation_token: &CancellationToken,
        cache_dir: &CacheDir,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError> {
        let Self(download_url) = self;
        let url = download_url.as_str();
        let cached = CachedDownloadedModel::new(cache_dir, url)
            .map_err(ModelSourceError::CacheDirectoryUnresolvable)?;
        let is_cached = cached.is_cached().await.map_err(|io_error| {
            self.fail_cache_access(&slot_aggregated_status, &cached.cache_file_path, io_error)
        })?;

        if is_cached {
            slot_aggregated_status.set_download_status(ModelDownloadStatus::NotDownloading);
            slot_aggregated_status
                .register_fix(&AgentIssueFix::ModelDownloadCompleted(self.model_path()));

            return Ok(DesiredModelResolution::Resolved(cached.cache_file_path));
        }

        cached
            .ensure_cache_subdir_exists()
            .await
            .map_err(|io_error| {
                self.fail_cache_access(&slot_aggregated_status, &cached.cache_subdir, io_error)
            })?;

        let _download_lock = match cached.try_acquire_download_lock() {
            Ok(DownloadLockAcquisition::Acquired(download_lock)) => download_lock,
            Ok(DownloadLockAcquisition::HeldByAnotherProcess) => {
                slot_aggregated_status
                    .fail_download(AgentIssue::CacheCannotAcquireLock(self.model_path()));

                return Err(ModelSourceError::DownloadLockHeldByAnotherProcess {
                    url: url.to_owned(),
                });
            }
            Err(io_error) => {
                return Err(self.fail_cache_access(
                    &slot_aggregated_status,
                    &cached.lock_file_path,
                    io_error,
                ));
            }
        };

        let download_progress = UrlDownloadProgress {
            model_path: self.model_path(),
            slot_aggregated_status: slot_aggregated_status.clone(),
        };
        let download_outcome = DownloadManager::new()
            .map_err(ModelSourceError::DownloaderUnavailable)?
            .download(
                cancellation_token,
                download_url,
                &cached.cache_file_path,
                &|progress| download_progress.apply(progress),
            )
            .await
            .map_err(|download_error| {
                self.fail_download(&slot_aggregated_status, download_error)
            })?;

        match download_outcome {
            DownloadOutcome::Completed => {
                Ok(DesiredModelResolution::Resolved(cached.cache_file_path))
            }
            DownloadOutcome::Cancelled => Ok(DesiredModelResolution::Cancelled),
        }
    }
}

#[async_trait]
impl ResolvesModelSource for UrlModelSource {
    async fn resolve(
        &self,
        cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError> {
        self.resolve_into_cache(
            cancellation_token,
            &CacheDir::from_process_env(),
            slot_aggregated_status,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::sync::Arc;

    use tempfile::TempDir;
    use tokio::fs::create_dir;
    use tokio::fs::read;
    use tokio::fs::write;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_cache_dir::cache_dir::CacheDir;
    use paddler_cache_dir::cache_dir_error::CacheDirError;
    use paddler_cache_dir::cached_downloaded_model::CachedDownloadedModel;
    use paddler_download_manager::download_error::DownloadError;
    use paddler_download_manager::download_url::DownloadUrl;
    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;

    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::model_source_error::ModelSourceError;
    use crate::url_model_source::UrlModelSource;

    fn url_model_source(url: &str) -> UrlModelSource {
        UrlModelSource(DownloadUrl::parse(url).unwrap())
    }

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    fn cache_dir_at(path: &Path) -> CacheDir {
        CacheDir {
            explicit: Some(path.to_path_buf()),
            home: None,
            xdg: None,
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

        let resolution = url_model_source(url_string)
            .resolve_into_cache(&CancellationToken::new(), &cache_dir, fresh_status())
            .await
            .unwrap();

        assert!(matches!(
            resolution,
            DesiredModelResolution::Resolved(resolved_path) if resolved_path == cached.cache_file_path
        ));
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
        let result = url_model_source(url_string)
            .resolve_into_cache(&CancellationToken::new(), &cache_dir, status.clone())
            .await;

        assert!(matches!(
            result,
            Err(ModelSourceError::DownloadLockHeldByAnotherProcess { url }) if url == url_string
        ));
        assert!(
            status.has_issue(&AgentIssue::CacheCannotAcquireLock(ModelPath {
                model_path: url_string.to_owned(),
            }))
        );
    }

    #[tokio::test]
    async fn cache_subdir_creation_failure_registers_model_cache_is_corrupted() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/subdir-blocked.gguf";
        let subdir_path = directory.path().join("downloaded-models");
        symlink(directory.path().join("missing-target"), &subdir_path).unwrap();

        let status = fresh_status();
        let result = url_model_source(url_string)
            .resolve_into_cache(&CancellationToken::new(), &cache_dir, status.clone())
            .await;

        assert!(matches!(
            result,
            Err(ModelSourceError::DownloadFailed {
                source: DownloadError::Io { path, .. },
                ..
            }) if path == subdir_path
        ));
        assert!(
            status.has_issue_like(|issue| matches!(issue, AgentIssue::ModelCacheIsCorrupted(_)))
        );
    }

    #[tokio::test]
    async fn lock_open_io_error_registers_model_cache_is_corrupted() {
        let directory = TempDir::new().unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/lock-as-directory.gguf";
        let cached = CachedDownloadedModel::new(&cache_dir, url_string).unwrap();
        cached.ensure_cache_subdir_exists().await.unwrap();
        create_dir(&cached.lock_file_path).await.unwrap();

        let status = fresh_status();
        let result = url_model_source(url_string)
            .resolve_into_cache(&CancellationToken::new(), &cache_dir, status.clone())
            .await;

        assert!(matches!(
            result,
            Err(ModelSourceError::DownloadFailed {
                source: DownloadError::Io { path, .. },
                ..
            }) if path == cached.lock_file_path
        ));
        assert!(
            status.has_issue_like(|issue| matches!(issue, AgentIssue::ModelCacheIsCorrupted(_)))
        );
    }

    #[tokio::test]
    async fn a_file_blocking_the_cache_subdir_registers_model_cache_is_corrupted() {
        let directory = TempDir::new().unwrap();
        write(directory.path().join("downloaded-models"), b"blocker")
            .await
            .unwrap();
        let cache_dir = cache_dir_at(directory.path());
        let url_string = "https://host.example/blocked.gguf";
        let cache_file_path = CachedDownloadedModel::new(&cache_dir, url_string)
            .unwrap()
            .cache_file_path;

        let status = fresh_status();
        let result = url_model_source(url_string)
            .resolve_into_cache(&CancellationToken::new(), &cache_dir, status.clone())
            .await;

        assert!(matches!(
            result,
            Err(ModelSourceError::DownloadFailed {
                source: DownloadError::Io { path, .. },
                ..
            }) if path == cache_file_path
        ));
        assert!(
            status.has_issue_like(|issue| matches!(issue, AgentIssue::ModelCacheIsCorrupted(_)))
        );
    }

    fn unresolvable_cache_dir() -> CacheDir {
        CacheDir {
            explicit: None,
            home: None,
            xdg: None,
        }
    }

    #[tokio::test]
    async fn cache_path_resolution_failure_propagates_error() {
        let url_string = "https://host.example/unresolvable.gguf";

        let result = url_model_source(url_string)
            .resolve_into_cache(
                &CancellationToken::new(),
                &unresolvable_cache_dir(),
                fresh_status(),
            )
            .await;

        assert_eq!(
            result.err().as_ref().map(discriminant),
            Some(discriminant(&ModelSourceError::CacheDirectoryUnresolvable(
                CacheDirError::HomeVariableUnset
            )))
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

        let resolution = url_model_source(&url_string)
            .resolve_into_cache(&CancellationToken::new(), &cache_dir, fresh_status())
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

        let result = url_model_source(&url_string)
            .resolve_into_cache(&cancellation_token, &cache_dir, status.clone())
            .await;

        assert!(
            result.is_ok_and(|resolution| resolution == DesiredModelResolution::Cancelled),
            "a cancelled download must report cancellation as an outcome, not an error"
        );
        assert!(
            status.make_snapshot().status.issues.is_empty(),
            "a cancelled download must not register a slot issue"
        );
    }
}
