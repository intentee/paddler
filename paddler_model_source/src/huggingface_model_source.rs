use std::sync::Arc;

use hf_hub::HFClient;
use hf_hub::HFError;
use log::warn;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_cache_dir::download_lock_wait_outcome::DownloadLockWaitOutcome;
use paddler_cache_dir::wait_for_download_lock_retry::wait_for_download_lock_retry;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::model_download_status::ModelDownloadStatus;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::huggingface_download_progress::HuggingFaceDownloadProgress;
use crate::huggingface_hub::HuggingFaceHub;
use crate::huggingface_model_reference::HuggingFaceModelReference;
use crate::model_source_error::ModelSourceError;
use crate::resolves_model_source::ResolvesModelSource;

const LOCK_RETRY_TIMEOUT: Duration = Duration::from_secs(10);

pub struct HuggingFaceModelSource {
    pub hub: HuggingFaceHub,
    pub reference: HuggingFaceModelReference,
}

impl HuggingFaceModelSource {
    async fn resolve_through(
        &self,
        client: &HFClient,
        model_path: ModelPath,
        cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError> {
        let reference = &self.reference;
        let repository = reference.repository(client);

        match reference.cached_path(&repository).await {
            Ok(cached_path) => {
                slot_aggregated_status.set_download_status(ModelDownloadStatus::NotDownloading);

                return Ok(DesiredModelResolution::Resolved(cached_path));
            }
            Err(HFError::EntryNotFound { .. } | HFError::LocalEntryNotFound { .. }) => {}
            Err(source) => {
                return Err(ModelSourceError::HuggingFaceCacheLookupFailed {
                    model_path: model_path.model_path,
                    source,
                });
            }
        }

        let resolve_ended = CancellationToken::new();
        let _resolve_ended_on_return = resolve_ended.clone().drop_guard();
        let Some(download_result) = cancellation_token
            .run_until_cancelled(
                repository
                    .download_file()
                    .filename(reference.filename.clone())
                    .revision(reference.revision.clone())
                    .progress(HuggingFaceDownloadProgress {
                        model_path: model_path.clone(),
                        resolve_ended,
                        slot_aggregated_status: slot_aggregated_status.clone(),
                    })
                    .send(),
            )
            .await
        else {
            return Ok(DesiredModelResolution::Cancelled);
        };

        match download_result {
            Ok(resolved_filename) => {
                slot_aggregated_status
                    .register_fix(&AgentIssueFix::HuggingFaceDownloadedModel(model_path));

                Ok(DesiredModelResolution::Resolved(resolved_filename))
            }
            Err(HFError::CacheLockTimeout { path }) => {
                let lock_path = path.display().to_string();

                slot_aggregated_status.register_desired_state_issue(
                    AgentIssue::HuggingFaceCannotAcquireLock(HuggingFaceDownloadLock {
                        lock_path: lock_path.clone(),
                        model_path,
                    }),
                );

                warn!(
                    "Waiting to acquire download lock for '{lock_path}'. Sleeping for {LOCK_RETRY_TIMEOUT:?}"
                );

                match wait_for_download_lock_retry(
                    cancellation_token,
                    LOCK_RETRY_TIMEOUT,
                    lock_path,
                )
                .await
                {
                    DownloadLockWaitOutcome::Cancelled => Ok(DesiredModelResolution::Cancelled),
                    DownloadLockWaitOutcome::LockStillUnavailable { lock_path } => {
                        Err(ModelSourceError::HuggingFaceDownloadLockUnavailable { lock_path })
                    }
                }
            }
            Err(
                HFError::EntryNotFound { .. }
                | HFError::RepoNotFound { .. }
                | HFError::RevisionNotFound { .. },
            ) => {
                slot_aggregated_status.register_desired_state_issue(
                    AgentIssue::HuggingFaceModelDoesNotExist(model_path.clone()),
                );

                Err(ModelSourceError::HuggingFaceModelMissing {
                    model_path: model_path.model_path,
                })
            }
            Err(HFError::AuthRequired { .. } | HFError::Forbidden { .. }) => {
                slot_aggregated_status.register_desired_state_issue(
                    AgentIssue::HuggingFacePermissions(model_path.clone()),
                );

                Err(ModelSourceError::HuggingFacePermissionDenied {
                    model_path: model_path.model_path,
                })
            }
            Err(source) => Err(ModelSourceError::HuggingFaceDownloadFailed {
                model_path: model_path.model_path,
                source,
            }),
        }
    }
}

impl ResolvesModelSource for HuggingFaceModelSource {
    async fn resolve(
        &self,
        cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError> {
        let model_path = self.reference.model_path();

        if slot_aggregated_status.has_issue(&AgentIssue::HuggingFaceModelDoesNotExist(
            model_path.clone(),
        )) {
            return Err(ModelSourceError::HuggingFaceModelKnownToBeMissing {
                model_path: model_path.model_path,
            });
        }

        let client = self
            .hub
            .start_client(&model_path, &slot_aggregated_status)?;

        self.resolve_through(
            &client,
            model_path,
            cancellation_token,
            slot_aggregated_status,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::fs::create_dir_all;
    use std::fs::read;
    use std::fs::write;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;

    use hf_hub::HFClient;
    use hf_hub::HFError;
    use reqwest::StatusCode;
    use tempfile::TempDir;
    use tempfile::tempdir;
    use tokio::join;
    use tokio_util::sync::CancellationToken;
    use url::ParseError;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
    use paddler_messaging::model_download_status::ModelDownloadStatus;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;
    use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

    use super::HuggingFaceModelSource;
    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::huggingface_cache_location::HuggingFaceCacheLocation;
    use crate::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
    use crate::huggingface_hub::HuggingFaceHub;
    use crate::huggingface_model_reference::HuggingFaceModelReference;
    use crate::model_source::ModelSource;
    use crate::model_source_error::ModelSourceError;
    use crate::resolves_model_source::ResolvesModelSource as _;

    const HUB_COMMIT_HASH: &str = "hub-commit";
    const HUB_ETAG: &str = "hub-etag";
    const HUB_MODEL_BYTES: &[u8] = b"hub model bytes";
    const REPOSITORY_CACHE_FOLDER: &str = "models--owner--repo";

    struct HubCache {
        cache_dir: TempDir,
    }

    impl HubCache {
        fn new() -> Self {
            Self {
                cache_dir: tempdir().unwrap(),
            }
        }

        fn client_of(&self, hub: &LocalHttpFixture) -> HFClient {
            HFClient::builder()
                .cache_dir(self.cache_dir.path())
                .endpoint(hub.url(""))
                .build()
                .unwrap()
        }

        fn model_source_downloading_from(&self, hub: &LocalHttpFixture) -> HuggingFaceModelSource {
            HuggingFaceModelSource {
                hub: HuggingFaceHub {
                    cache_location: HuggingFaceCacheLocation::Directory(
                        self.cache_dir.path().to_path_buf(),
                    ),
                    endpoint: hub.url(""),
                },
                reference: uncached_model_reference(),
            }
        }

        fn blob_lock_path(&self) -> PathBuf {
            self.cache_dir
                .path()
                .join(".locks")
                .join(REPOSITORY_CACHE_FOLDER)
                .join(format!("{HUB_ETAG}.lock"))
        }

        fn hold_blob_lock(&self) -> File {
            let blob_lock_path = self.blob_lock_path();

            create_dir_all(blob_lock_path.parent().unwrap()).unwrap();

            let held_blob_lock = File::create(blob_lock_path).unwrap();

            held_blob_lock.lock().unwrap();

            held_blob_lock
        }
    }

    fn uncached_model_reference() -> HuggingFaceModelReference {
        HuggingFaceModelReference {
            filename: "model.gguf".to_owned(),
            repository_name: "repo".to_owned(),
            repository_owner: "owner".to_owned(),
            revision: "main".to_owned(),
        }
    }

    fn hub_model_file() -> FixtureResponse {
        FixtureResponse::HuggingFaceFile {
            body: HUB_MODEL_BYTES.to_vec(),
            commit_hash: HUB_COMMIT_HASH.to_owned(),
            etag: HUB_ETAG.to_owned(),
        }
    }

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    fn offline_hub_of(cache_dir: &Path) -> HuggingFaceHub {
        HuggingFaceHub {
            cache_location: HuggingFaceCacheLocation::Directory(cache_dir.to_path_buf()),
            endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
        }
    }

    fn cache_model_file_at_revision(
        cache_dir: &Path,
        revision: &str,
        commit_hash: &str,
        filename: &str,
    ) -> PathBuf {
        let repository_cache = cache_dir.join(REPOSITORY_CACHE_FOLDER);
        let snapshot_dir = repository_cache.join("snapshots").join(commit_hash);
        let model_file = snapshot_dir.join(filename);

        create_dir_all(repository_cache.join("refs")).unwrap();
        write(repository_cache.join("refs").join(revision), commit_hash).unwrap();
        create_dir_all(&snapshot_dir).unwrap();
        File::create(&model_file).unwrap();

        model_file
    }

    #[tokio::test]
    async fn resolves_the_cached_file_of_the_requested_revision() {
        let cache_dir = tempdir().unwrap();

        cache_model_file_at_revision(cache_dir.path(), "main", "main-commit", "model.gguf");

        let requested_revision_file =
            cache_model_file_at_revision(cache_dir.path(), "v2", "v2-commit", "model.gguf");
        let resolution = HuggingFaceModelSource {
            hub: offline_hub_of(cache_dir.path()),
            reference: HuggingFaceModelReference {
                revision: "v2".to_owned(),
                ..uncached_model_reference()
            },
        }
        .resolve(&CancellationToken::new(), fresh_status())
        .await
        .unwrap();

        assert_eq!(
            resolution,
            DesiredModelResolution::Resolved(requested_revision_file)
        );
    }

    #[tokio::test]
    async fn resolves_a_cached_file_from_a_uri_that_percent_encodes_its_name() {
        let cache_dir = tempdir().unwrap();
        let cached_file =
            cache_model_file_at_revision(cache_dir.path(), "main", "main-commit", "my modèle.gguf");
        let reference = HuggingFaceModelReference {
            filename: "my modèle.gguf".to_owned(),
            ..uncached_model_reference()
        };

        assert_eq!(
            ModelSource::parse(
                "https://huggingface.co/owner/repo/resolve/main/my%20mod%C3%A8le.gguf",
                &SlotAggregatedStatus::new(1),
            )
            .unwrap(),
            ModelSource::HuggingFace(reference.clone())
        );
        assert_eq!(
            HuggingFaceModelSource {
                hub: offline_hub_of(cache_dir.path()),
                reference,
            }
            .resolve(&CancellationToken::new(), fresh_status())
            .await
            .unwrap(),
            DesiredModelResolution::Resolved(cached_file)
        );
    }

    #[tokio::test]
    async fn a_client_without_a_cache_cannot_look_up_cached_models() {
        let cache_dir = tempdir().unwrap();
        let resolution = HuggingFaceModelSource {
            hub: offline_hub_of(cache_dir.path()),
            reference: uncached_model_reference(),
        }
        .resolve_through(
            &HFClient::builder().cache_enabled(false).build().unwrap(),
            uncached_model_reference().model_path(),
            &CancellationToken::new(),
            fresh_status(),
        )
        .await;

        assert_eq!(
            resolution
                .err()
                .map(|resolution_error| resolution_error.to_string()),
            Some(
                ModelSourceError::HuggingFaceCacheLookupFailed {
                    model_path: uncached_model_reference().model_path().model_path,
                    source: HFError::CacheNotEnabled,
                }
                .to_string()
            )
        );
    }

    #[tokio::test]
    async fn a_malformed_endpoint_keeps_the_hub_client_from_starting_until_it_is_corrected() {
        let hub_cache = HubCache::new();
        let hub = LocalHttpFixture::start(hub_model_file()).await.unwrap();
        let slot_aggregated_status = fresh_status();
        let client_cannot_start =
            AgentIssue::HuggingFaceClientCannotStart(uncached_model_reference().model_path());

        let resolution_with_a_malformed_endpoint = HuggingFaceModelSource {
            hub: HuggingFaceHub {
                cache_location: HuggingFaceCacheLocation::Directory(
                    hub_cache.cache_dir.path().to_path_buf(),
                ),
                endpoint: "not an endpoint".to_owned(),
            },
            reference: uncached_model_reference(),
        }
        .resolve(&CancellationToken::new(), slot_aggregated_status.clone())
        .await;
        let issue_reported_with_a_malformed_endpoint =
            slot_aggregated_status.has_issue(&client_cannot_start);
        let resolution_with_a_corrected_endpoint = hub_cache
            .model_source_downloading_from(&hub)
            .resolve(&CancellationToken::new(), slot_aggregated_status.clone())
            .await;

        assert_eq!(
            resolution_with_a_malformed_endpoint
                .err()
                .map(|resolution_error| resolution_error.to_string()),
            Some(
                ModelSourceError::HuggingFaceClientUnavailable(HFError::Url(
                    ParseError::RelativeUrlWithoutBase
                ))
                .to_string()
            )
        );
        assert!(issue_reported_with_a_malformed_endpoint);
        assert_eq!(
            resolution_with_a_corrected_endpoint.unwrap(),
            DesiredModelResolution::Resolved(
                uncached_model_reference()
                    .cached_path(&uncached_model_reference().repository(&hub_cache.client_of(&hub)))
                    .await
                    .unwrap()
            )
        );
        assert!(!slot_aggregated_status.has_issue(&client_cannot_start));
    }

    #[tokio::test]
    async fn downloads_an_uncached_file_from_its_hub_into_the_cache() {
        let hub_cache = HubCache::new();
        let hub = LocalHttpFixture::start(hub_model_file()).await.unwrap();
        let slot_aggregated_status = fresh_status();

        let resolution = hub_cache
            .model_source_downloading_from(&hub)
            .resolve(&CancellationToken::new(), slot_aggregated_status.clone())
            .await
            .unwrap();
        let cached_path = uncached_model_reference()
            .cached_path(&uncached_model_reference().repository(&hub_cache.client_of(&hub)))
            .await
            .unwrap();

        assert_eq!(
            resolution,
            DesiredModelResolution::Resolved(cached_path.clone())
        );
        assert_eq!(read(cached_path).unwrap(), HUB_MODEL_BYTES);
        assert_eq!(
            slot_aggregated_status
                .make_snapshot()
                .status
                .download_status,
            ModelDownloadStatus::NotDownloading
        );
    }

    #[tokio::test]
    async fn a_file_missing_from_its_hub_is_not_requested_again() {
        let hub_cache = HubCache::new();
        let hub = LocalHttpFixture::start(FixtureResponse::Status(StatusCode::NOT_FOUND))
            .await
            .unwrap();
        let model_source = hub_cache.model_source_downloading_from(&hub);
        let slot_aggregated_status = fresh_status();
        let model_path = uncached_model_reference().model_path().model_path;

        let first_resolution = model_source
            .resolve(&CancellationToken::new(), slot_aggregated_status.clone())
            .await;
        let second_resolution = model_source
            .resolve(&CancellationToken::new(), slot_aggregated_status)
            .await;

        assert!(matches!(
            first_resolution,
            Err(ModelSourceError::HuggingFaceModelMissing { model_path: missing_model_path })
                if missing_model_path == model_path
        ));
        assert!(matches!(
            second_resolution,
            Err(ModelSourceError::HuggingFaceModelKnownToBeMissing { model_path: missing_model_path })
                if missing_model_path == model_path
        ));
        assert_eq!(hub.request_count(), 1);
    }

    #[tokio::test]
    async fn a_file_its_hub_refuses_to_share_reports_missing_permissions() {
        let hub_cache = HubCache::new();
        let hub = LocalHttpFixture::start(FixtureResponse::Status(StatusCode::FORBIDDEN))
            .await
            .unwrap();
        let slot_aggregated_status = fresh_status();

        let resolution = hub_cache
            .model_source_downloading_from(&hub)
            .resolve(&CancellationToken::new(), slot_aggregated_status.clone())
            .await;

        assert_eq!(
            resolution
                .err()
                .map(|resolution_error| resolution_error.to_string()),
            Some(
                ModelSourceError::HuggingFacePermissionDenied {
                    model_path: uncached_model_reference().model_path().model_path,
                }
                .to_string()
            )
        );
        assert!(
            slot_aggregated_status.has_issue(&AgentIssue::HuggingFacePermissions(
                uncached_model_reference().model_path()
            ))
        );
    }

    #[tokio::test]
    async fn a_hub_failing_with_a_server_error_fails_the_download() {
        let hub_cache = HubCache::new();
        let hub =
            LocalHttpFixture::start(FixtureResponse::Status(StatusCode::INTERNAL_SERVER_ERROR))
                .await
                .unwrap();

        let resolution = hub_cache
            .model_source_downloading_from(&hub)
            .resolve(&CancellationToken::new(), fresh_status())
            .await;

        assert!(matches!(
            resolution,
            Err(ModelSourceError::HuggingFaceDownloadFailed {
                source: HFError::Http { context },
                ..
            }) if context.status == StatusCode::INTERNAL_SERVER_ERROR
        ));
    }

    #[tokio::test]
    async fn a_hub_answer_without_file_metadata_fails_the_download() {
        let hub_cache = HubCache::new();
        let hub = LocalHttpFixture::start(FixtureResponse::Ok(HUB_MODEL_BYTES.to_vec()))
            .await
            .unwrap();

        let resolution = hub_cache
            .model_source_downloading_from(&hub)
            .resolve(&CancellationToken::new(), fresh_status())
            .await;

        assert!(matches!(
            resolution,
            Err(ModelSourceError::HuggingFaceDownloadFailed {
                source: HFError::MalformedResponse { what, .. },
                model_path,
            }) if what == "missing ETag header"
                && model_path == uncached_model_reference().model_path().model_path
        ));
    }

    #[tokio::test]
    async fn a_blob_locked_by_another_process_stays_unavailable_after_the_lock_retry_wait() {
        let hub_cache = HubCache::new();
        let hub = LocalHttpFixture::start(hub_model_file()).await.unwrap();
        let _held_blob_lock = hub_cache.hold_blob_lock();
        let slot_aggregated_status = fresh_status();
        let lock_path = hub_cache.blob_lock_path().display().to_string();

        let resolution = hub_cache
            .model_source_downloading_from(&hub)
            .resolve(&CancellationToken::new(), slot_aggregated_status.clone())
            .await;

        assert!(matches!(
            resolution,
            Err(ModelSourceError::HuggingFaceDownloadLockUnavailable { lock_path: unavailable_lock_path })
                if unavailable_lock_path == lock_path
        ));
        assert!(
            slot_aggregated_status.has_issue(&AgentIssue::HuggingFaceCannotAcquireLock(
                HuggingFaceDownloadLock {
                    lock_path,
                    model_path: uncached_model_reference().model_path(),
                }
            ))
        );
    }

    #[tokio::test]
    async fn a_resolution_cancelled_while_waiting_for_a_locked_blob_is_cancelled() {
        let hub_cache = HubCache::new();
        let hub = LocalHttpFixture::start(hub_model_file()).await.unwrap();
        let model_source = hub_cache.model_source_downloading_from(&hub);
        let _held_blob_lock = hub_cache.hold_blob_lock();
        let slot_aggregated_status = fresh_status();
        let mut status_updates = slot_aggregated_status.subscribe_to_updates();
        let cancellation_token = CancellationToken::new();

        let (resolution, ()) = join!(
            model_source.resolve(&cancellation_token, slot_aggregated_status.clone()),
            async {
                status_updates
                    .wait_for(|()| {
                        slot_aggregated_status.has_issue(&AgentIssue::HuggingFaceCannotAcquireLock(
                            HuggingFaceDownloadLock {
                                lock_path: hub_cache.blob_lock_path().display().to_string(),
                                model_path: uncached_model_reference().model_path(),
                            },
                        ))
                    })
                    .await
                    .unwrap();
                cancellation_token.cancel();
            }
        );

        assert_eq!(resolution.unwrap(), DesiredModelResolution::Cancelled);
    }

    #[tokio::test]
    async fn a_cancelled_resolution_of_an_uncached_model_requests_nothing_from_its_hub() {
        let hub_cache = HubCache::new();
        let hub = LocalHttpFixture::start(hub_model_file()).await.unwrap();
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        assert_eq!(
            hub_cache
                .model_source_downloading_from(&hub)
                .resolve(&cancellation_token, fresh_status())
                .await
                .unwrap(),
            DesiredModelResolution::Cancelled
        );
        assert_eq!(hub.request_count(), 0);
    }
}
