use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use hf_hub::Cache;
use hf_hub::Repo;
use hf_hub::RepoType;
use hf_hub::api::tokio::ApiBuilder;
use hf_hub::api::tokio::ApiError;
use hf_hub::api::tokio::ApiRepo;
use log::info;
use log::warn;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use tokio::time::Duration;
use tokio::time::sleep;

use crate::agent_issue_fix::AgentIssueFix;
use crate::converts_to_applicable_state::ConvertsToApplicableState;
use crate::slot_aggregated_status::SlotAggregatedStatus;
use crate::slot_aggregated_status_download_progress::SlotAggregatedStatusDownloadProgress;

const LOCK_RETRY_TIMEOUT: Duration = Duration::from_secs(10);

async fn download_companion_mmproj_files(hf_repo: &ApiRepo, hf_cache: &Cache, repo_id: &str) {
    let cache_repo = hf_cache.repo(Repo::new(repo_id.to_owned(), RepoType::Model));
    let repo_info = match hf_repo.info().await {
        Ok(repo_info) => repo_info,
        Err(err) => {
            info!("Could not fetch repo info for mmproj discovery: {err}");

            return;
        }
    };

    for sibling in &repo_info.siblings {
        let sibling_filename = &sibling.rfilename;

        if !sibling_filename.contains("mmproj") || !sibling_filename.ends_with(".gguf") {
            continue;
        }

        if cache_repo.get(sibling_filename).is_some() {
            info!("Companion mmproj file already cached: {sibling_filename}");

            continue;
        }

        info!("Downloading companion mmproj file: {sibling_filename}");

        match hf_repo.download(sibling_filename).await {
            Ok(downloaded_path) => {
                info!("Downloaded mmproj file to: {}", downloaded_path.display());
            }
            Err(err) => {
                warn!("Failed to download mmproj file '{sibling_filename}': {err}");
            }
        }
    }
}

#[async_trait]
impl ConvertsToApplicableState for AgentDesiredModel {
    type ApplicableState = PathBuf;
    type Context = Arc<SlotAggregatedStatus>;

    async fn to_applicable_state(
        &self,
        slot_aggregated_status: Self::Context,
    ) -> Result<Option<Self::ApplicableState>> {
        Ok(match self {
            AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
                filename,
                repo_id,
                revision,
            }) => {
                let model_path = format!("{repo_id}/{revision}/{filename}");

                if slot_aggregated_status.has_issue(&AgentIssue::HuggingFaceModelDoesNotExist(
                    model_path.clone(),
                )) {
                    return Err(anyhow!(
                        "Model '{model_path}' does not exist on Hugging Face. Not attempting to download it again."
                    ));
                }

                let hf_cache = Cache::default();
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
                    download_companion_mmproj_files(&hf_repo, &hf_cache, repo_id).await;

                    return Ok(Some(cached_path));
                }

                let weights_filename = match hf_repo
                    .download_with_progress(
                        filename,
                        SlotAggregatedStatusDownloadProgress::new(slot_aggregated_status.clone()),
                    )
                    .await
                {
                    Ok(resolved_filename) => {
                        slot_aggregated_status
                            .register_fix(AgentIssueFix::HuggingFaceDownloadedModel);

                        resolved_filename
                    }
                    Err(ApiError::LockAcquisition(lock_path)) => {
                        slot_aggregated_status.register_issue(
                            AgentIssue::HuggingFaceCannotAcquireLock(
                                lock_path.display().to_string(),
                            ),
                        );

                        warn!(
                            "Waiting to acquire download lock for '{}'. Sleeping for {} secs",
                            lock_path.display(),
                            LOCK_RETRY_TIMEOUT.as_secs()
                        );

                        sleep(LOCK_RETRY_TIMEOUT).await;

                        return Err(anyhow!(
                            "Failed to acquire download lock '{}'. Is more than one agent running on this machine?",
                            lock_path.display()
                        ));
                    }
                    Err(ApiError::RequestError(reqwest_error)) => match reqwest_error.status() {
                        Some(reqwest::StatusCode::NOT_FOUND) => {
                            slot_aggregated_status.register_issue(
                                AgentIssue::HuggingFaceModelDoesNotExist(model_path.clone()),
                            );

                            return Err(anyhow!(
                                "Model '{model_path}' does not exist on Hugging Face."
                            ));
                        }
                        _ => {
                            return Err(anyhow!(
                                "Failed to download model from Hugging Face: {}",
                                reqwest_error
                            ));
                        }
                    },
                    Err(err_other) => return Err(err_other.into()),
                };

                download_companion_mmproj_files(&hf_repo, &hf_cache, repo_id).await;

                Some(weights_filename)
            }
            AgentDesiredModel::LocalToAgent(path) => Some(PathBuf::from(path)),
            AgentDesiredModel::None => None,
        })
    }
}
