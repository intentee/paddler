use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use hf_hub::Cache;
use hf_hub::Repo;
use hf_hub::RepoType;
use hf_hub::api::tokio::ApiBuilder;
use hf_hub::api::tokio::ApiError;
use log::warn;
use tokio::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::hugging_face_download_lock::HuggingFaceDownloadLock;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

use crate::agent_issue_fix::AgentIssueFix;
use crate::desired_model_resolution::DesiredModelResolution;
use crate::resolves_model_source::ResolvesModelSource;
use crate::slot_aggregated_status::SlotAggregatedStatus;
use crate::slot_aggregated_status_download_progress::SlotAggregatedStatusDownloadProgress;

const LOCK_RETRY_TIMEOUT: Duration = Duration::from_secs(10);

pub struct HuggingFaceModelSource(pub HuggingFaceModelReference);

#[async_trait]
impl ResolvesModelSource for HuggingFaceModelSource {
    async fn resolve(
        &self,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
        cancellation_token: &CancellationToken,
    ) -> Result<DesiredModelResolution> {
        let HuggingFaceModelReference {
            filename,
            repo_id,
            revision,
        } = &self.0;
        let model_path = format!("{repo_id}/{revision}/{filename}");

        if slot_aggregated_status.has_issue(&AgentIssue::HuggingFaceModelDoesNotExist(ModelPath {
            model_path: model_path.clone(),
        })) {
            return Err(anyhow!(
                "Model '{model_path}' does not exist on Hugging Face. Not attempting to download it again."
            ));
        }

        let hf_cache = Cache::from_env();
        let hf_api = ApiBuilder::from_cache(hf_cache.clone()).build()?;
        let hf_repo = hf_api.repo(Repo::with_revision(
            repo_id.to_owned(),
            RepoType::Model,
            revision.to_owned(),
        ));

        if let Some(cached_path) = hf_cache
            .repo(Repo::new(repo_id.to_owned(), RepoType::Model))
            .get(filename)
        {
            slot_aggregated_status.reset_download();

            return Ok(DesiredModelResolution::Resolved(cached_path));
        }

        let Some(download_result) = cancellation_token
            .run_until_cancelled(hf_repo.download_with_progress(
                filename,
                SlotAggregatedStatusDownloadProgress::new(slot_aggregated_status.clone()),
            ))
            .await
        else {
            return Err(anyhow!(
                "Hugging Face model download for '{model_path}' was cancelled"
            ));
        };

        match download_result {
            Ok(resolved_filename) => {
                slot_aggregated_status.register_fix(&AgentIssueFix::HuggingFaceDownloadedModel(
                    ModelPath { model_path },
                ));

                Ok(DesiredModelResolution::Resolved(resolved_filename))
            }
            Err(ApiError::LockAcquisition(lock_path)) => {
                slot_aggregated_status.register_issue(AgentIssue::HuggingFaceCannotAcquireLock(
                    HuggingFaceDownloadLock {
                        lock_path: lock_path.display().to_string(),
                        model_path: ModelPath { model_path },
                    },
                ));

                warn!(
                    "Waiting to acquire download lock for '{}'. Sleeping for {} secs",
                    lock_path.display(),
                    LOCK_RETRY_TIMEOUT.as_secs()
                );

                cancellation_token
                    .run_until_cancelled(sleep(LOCK_RETRY_TIMEOUT))
                    .await;

                Err(anyhow!(
                    "Failed to acquire download lock '{}'. Is more than one agent running on this machine?",
                    lock_path.display()
                ))
            }
            Err(ApiError::RequestError(reqwest_error)) => match reqwest_error.status() {
                Some(reqwest::StatusCode::NOT_FOUND) => {
                    slot_aggregated_status.register_issue(
                        AgentIssue::HuggingFaceModelDoesNotExist(ModelPath {
                            model_path: model_path.clone(),
                        }),
                    );

                    Err(anyhow!(
                        "Model '{model_path}' does not exist on Hugging Face."
                    ))
                }
                Some(reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::UNAUTHORIZED) => {
                    slot_aggregated_status.register_issue(AgentIssue::HuggingFacePermissions(
                        ModelPath {
                            model_path: model_path.clone(),
                        },
                    ));

                    Err(anyhow!(
                        "You do not have enough permissions to download '{model_path}' from Hugging Face."
                    ))
                }
                _ => Err(anyhow!(
                    "Failed to download model from Hugging Face: {reqwest_error}"
                )),
            },
            Err(err_other) => Err(err_other.into()),
        }
    }
}
