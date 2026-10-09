use std::sync::Arc;

use async_trait::async_trait;
use hf_hub::Cache;
use hf_hub::Repo;
use hf_hub::RepoType;
use hf_hub::api::tokio::ApiBuilder;
use hf_hub::api::tokio::ApiError;
use log::warn;
use reqwest::StatusCode;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_cache_dir::download_lock_wait_outcome::DownloadLockWaitOutcome;
use paddler_cache_dir::wait_for_download_lock_retry::wait_for_download_lock_retry;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
use paddler_messaging::model_download_status::ModelDownloadStatus;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::huggingface_download_progress::HuggingFaceDownloadProgress;
use crate::huggingface_model_reference::HuggingFaceModelReference;
use crate::model_source_error::ModelSourceError;
use crate::resolves_model_source::ResolvesModelSource;

const LOCK_RETRY_TIMEOUT: Duration = Duration::from_secs(10);

pub struct HuggingFaceModelSource {
    pub cache: Cache,
    pub reference: HuggingFaceModelReference,
}

#[async_trait]
impl ResolvesModelSource for HuggingFaceModelSource {
    async fn resolve(
        &self,
        cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError> {
        let Self { cache, reference } = self;
        let HuggingFaceModelReference {
            filename,
            repo_id,
            revision,
        } = reference;
        let model_path = reference.model_path();

        if slot_aggregated_status.has_issue(&AgentIssue::HuggingFaceModelDoesNotExist(
            model_path.clone(),
        )) {
            return Err(ModelSourceError::HuggingFaceModelKnownToBeMissing {
                model_path: model_path.model_path,
            });
        }

        let repo = Repo::with_revision(repo_id.to_owned(), RepoType::Model, revision.to_owned());

        if let Some(cached_path) = cache.repo(repo.clone()).get(filename) {
            slot_aggregated_status.set_download_status(ModelDownloadStatus::NotDownloading);

            return Ok(DesiredModelResolution::Resolved(cached_path));
        }

        let hf_repo = ApiBuilder::from_cache(cache.clone())
            .build()
            .map_err(ModelSourceError::HuggingFaceClientUnavailable)?
            .repo(repo);
        let Some(download_result) = cancellation_token
            .run_until_cancelled(hf_repo.download_with_progress(
                filename,
                HuggingFaceDownloadProgress {
                    model_path: model_path.clone(),
                    slot_aggregated_status: slot_aggregated_status.clone(),
                },
            ))
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
            Err(ApiError::LockAcquisition(lock_path)) => {
                let lock_path = lock_path.display().to_string();

                slot_aggregated_status.register_issue(AgentIssue::HuggingFaceCannotAcquireLock(
                    HuggingFaceDownloadLock {
                        lock_path: lock_path.clone(),
                        model_path,
                    },
                ));

                warn!(
                    "Waiting to acquire download lock for '{lock_path}'. Sleeping for {} secs",
                    LOCK_RETRY_TIMEOUT.as_secs()
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
            Err(ApiError::RequestError(reqwest_error)) => match reqwest_error.status() {
                Some(StatusCode::NOT_FOUND) => {
                    slot_aggregated_status.register_issue(
                        AgentIssue::HuggingFaceModelDoesNotExist(model_path.clone()),
                    );

                    Err(ModelSourceError::HuggingFaceModelMissing {
                        model_path: model_path.model_path,
                    })
                }
                Some(StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED) => {
                    slot_aggregated_status
                        .register_issue(AgentIssue::HuggingFacePermissions(model_path.clone()));

                    Err(ModelSourceError::HuggingFacePermissionDenied {
                        model_path: model_path.model_path,
                    })
                }
                _other_status => Err(ModelSourceError::HuggingFaceDownloadFailed {
                    model_path: model_path.model_path,
                    source: ApiError::RequestError(reqwest_error),
                }),
            },
            Err(source) => Err(ModelSourceError::HuggingFaceDownloadFailed {
                model_path: model_path.model_path,
                source,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::fs::create_dir_all;
    use std::path::PathBuf;
    use std::sync::Arc;

    use hf_hub::Cache;
    use hf_hub::Repo;
    use hf_hub::RepoType;
    use tempfile::tempdir;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

    use super::HuggingFaceModelSource;
    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::huggingface_model_reference::HuggingFaceModelReference;
    use crate::resolves_model_source::ResolvesModelSource as _;

    fn cache_model_file_at_revision(cache: &Cache, revision: &str, commit_hash: &str) -> PathBuf {
        let repo = Repo::with_revision(
            "owner/repo".to_owned(),
            RepoType::Model,
            revision.to_owned(),
        );

        cache.repo(repo.clone()).create_ref(commit_hash).unwrap();

        let snapshot_dir = cache
            .path()
            .join(repo.folder_name())
            .join("snapshots")
            .join(commit_hash);

        create_dir_all(&snapshot_dir).unwrap();

        let model_file = snapshot_dir.join("model.gguf");

        File::create(&model_file).unwrap();

        model_file
    }

    #[tokio::test]
    async fn resolves_the_cached_file_of_the_requested_revision() {
        let cache_dir = tempdir().unwrap();
        let cache = Cache::new(cache_dir.path().to_path_buf());

        cache_model_file_at_revision(&cache, "main", "main-commit");

        let requested_revision_file = cache_model_file_at_revision(&cache, "v2", "v2-commit");
        let resolution = HuggingFaceModelSource {
            cache,
            reference: HuggingFaceModelReference {
                filename: "model.gguf".to_owned(),
                repo_id: "owner/repo".to_owned(),
                revision: "v2".to_owned(),
            },
        }
        .resolve(
            &CancellationToken::new(),
            Arc::new(SlotAggregatedStatus::new(1)),
        )
        .await
        .unwrap();

        assert!(matches!(
            resolution,
            DesiredModelResolution::Resolved(resolved_file) if resolved_file == requested_revision_file
        ));
    }
}
